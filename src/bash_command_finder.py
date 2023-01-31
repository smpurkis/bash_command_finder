import subprocess as sp
import sys
from pathlib import Path
from typing import Any, Dict, List

import pyperclip  # type: ignore
import requests
from colorama import init
import json
import click

import urllib3
from dotenv import load_dotenv
import os

load_dotenv()

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
init()

API_URL = "https://api-inference.huggingface.co/models/bigscience/bloom"
# API_URL = "https://api-inference.huggingface.co/models/bigscience/bloom-560m"
headers = {"Bearer": os.environ["HUGGING_FACE_API_KEY"]}


# Levenshtein distance function
def levenshtein_distance(s1: str, s2: str) -> int:
    if len(s1) < len(s2):
        return levenshtein_distance(s2, s1)

    # len(s1) >= len(s2)
    if len(s2) == 0:
        return len(s1)

    previous_row = range(len(s2) + 1)
    for i, c1 in enumerate(s1):
        current_row = [i + 1]
        for j, c2 in enumerate(s2):
            insertions = (
                previous_row[j + 1] + 1
            )  # j+1 instead of j since previous_row and current_row are one character longer
            deletions = current_row[j] + 1  # than s2
            substitutions = previous_row[j] + (c1 != c2)
            current_row.append(min(insertions, deletions, substitutions))
        previous_row = current_row

    return previous_row[-1]


def query(payload: dict) -> dict:
    response = requests.post(API_URL, headers=headers, json=payload)
    return response.json()


def load_json_cmd_examples() -> List[List[str]]:
    return json.load(open("cmd_examples.json"))


def save_json_cmd_examples(cmd_examples: List[List[str]]):
    cmd_examples = []
    for cmd in cmd_examples:
        cmd_examples.append([cmd[0], cmd[1]])

    cmd_examples_set = set(cmd_examples)
    cmd_examples = list(cmd_examples_set)
    json.dump(cmd_examples, open("cmd_examples.json", "w"), sort_keys=True, indent=4)


EXAMPLES_CONTEXT = "Linux bash command to accomplish the task"


def form_query_base(cmd_examples: List[List[str]]) -> List[str]:
    base_query_list = [EXAMPLES_CONTEXT]
    for cmd_eg in cmd_examples:

        base_query_list.append("")
        base_query_list.append(f"# {cmd_eg[0]}")
        base_query_list.append(cmd_eg[1])
    return base_query_list


def parse_code_grepper_answer(answer: str, debug: bool = False) -> str:
    # try:
    #     if "#" in answer:
    #         answer_lines = [l for l in answer.split("\n") if l != ""]
    #         answer = "\n".join([a for a in answer_lines if a[0] != "#"])
    # except Exception as e:
    #     if debug:
    #         sys.stdout.write(str(e))
    #     answer = ""
    return answer


def query_code_grepper(search: str, debug: bool = False) -> List[str]:
    api_version = 3
    query = f"bash {search}"
    if debug:
        sys.stdout.write(query)
        sys.stdout.write("\n")
    base_url = f"https://www.codegrepper.com/api/get_answers_1.php?v={api_version}&s={requests.utils.quote(query)}"

    resp = requests.get(
        base_url, verify=False
    )  # verify=True errs with certificate verify failed: unable to get local issuer certificate
    resp_json = resp.json()
    answers = [
        parse_code_grepper_answer(a["answer"], debug) for a in resp_json["answers"]
    ]
    answers = [a for a in answers if a != ""]
    return answers


def run_process(cmd: str, cwd: str = Path().as_posix()):
    process = sp.Popen(cmd, shell=True, stdout=sp.PIPE, cwd=cwd)
    for line in iter(process.stdout.readline, ""):
        sys.stdout.write(line.decode())
        poll = process.poll()
        if poll is not None:
            break


def parse_bloom_output(output: Dict[Any, Any], query_text: str, cmd_text: str) -> str:
    full_output = output[0]["generated_text"]
    output_split = full_output.split(query_text)
    answer_lines = []
    if len(output_split) == 2:
        answer_lines = output_split[1].split("\n")
    elif len(output_split) == 1:
        output_lines = output_split[0].split("\n")
        output_line_lev_dists = [
            levenshtein_distance(a, cmd_text) for a in output_lines
        ]
        last_query_line_index = output_line_lev_dists.index(min(output_line_lev_dists))
        answer_lines = output_lines[last_query_line_index + 1 :]
    if len(answer_lines) == 0:
        raise Exception("No command via Bloom could be found!")
    answer_line = [a for a in answer_lines if a != ""][0]
    answer_line = correct_answer_line(answer_line)
    return answer_line


def correct_answer_line(answer_line: str) -> str:
    answer_comps = answer_line.split()
    first_cmd = answer_comps[0]
    first_cmd = "".join(e for e in first_cmd if e.isalnum())
    answer_comps[0] = first_cmd
    corrected_answer_line = " ".join(answer_comps)
    return corrected_answer_line


def run_bloom_query(text: str, debug: bool = False) -> str:
    cmd_examples = load_json_cmd_examples()
    base_query_list = form_query_base(cmd_examples)
    base_query_list.append("")
    base_query_list.append(f"# {text}")
    query_text = "\n".join(base_query_list)
    if debug:
        sys.stdout.write(query_text + "\n")
    output = query({"inputs": query_text})
    answer_line = parse_bloom_output(output, query_text, text)
    return answer_line


def confirm_run_of_bloom_query(
    input_text: str, cmd_text: str, cmd_origin: str, to_clipboard: bool = False
) -> bool:
    sys.stdout.write(f"From {cmd_origin}: {cmd_text}")
    sys.stdout.write("\n")
    if to_clipboard:
        confirm_text = input("Would you like to copy this command to clipboard (y/n)? ")
    else:
        confirm_text = input("Is this command correct (y/n)? ")
    if confirm_text.lower() in ["y", "yes"]:
        cmd_examples = load_json_cmd_examples()
        cmd_examples.append([input_text, cmd_text])
        save_json_cmd_examples(cmd_examples)
        if to_clipboard:
            pyperclip.copy(cmd_text)
        return True
    else:
        return False


def check_cache(text: str):
    cmd_examples = load_json_cmd_examples()
    for cmd_eg in cmd_examples:
        if text == cmd_eg[0]:
            return cmd_eg[1]
    return None


@click.command()
@click.argument("search")
@click.option(
    "--debug",
    is_flag=True,
    help="show debug logging, i.e. raw query sent to bloom",
)
@click.option(
    "--disable_cache",
    is_flag=True,
    help="use the cache to reduce queries of previous searches",
)
@click.option(
    "--disable_bloom",
    is_flag=True,
    help="disable bloom search",
)
@click.option(
    "--disable_codegrepper",
    is_flag=True,
    help="disable code grepper search",
)
@click.option(
    "--disable_clipboard",
    is_flag=True,
    help="disable asking to copy to clipboard, will only print the suggestions",
)
def cmdline_main(
    search: str,
    debug: bool,
    disable_cache: bool,
    disable_bloom: bool,
    disable_codegrepper: bool,
    disable_clipboard: bool,
):
    """
    Find bash command by describing it in English.

    Please set HUGGING_FACE_API_KEY to be able to use Bloom search functionality
    E.g. "Bearer xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" from https://huggingface.co/bigscience/bloom


    Example usage

    - print "hello world" -> echo "hello world"

    - find all .log files in current directory -> find -name '*.log'
    """
    cmd_origin = "cache"
    cmd_text = None
    if not disable_cache:
        cmd_text = check_cache(search)
    if cmd_text is None:
        if disable_codegrepper:
            codegrepper_cmds = []
        else:
            codegrepper_cmds = query_code_grepper(search, debug)
        if len(codegrepper_cmds) > 1:
            cmd_text = codegrepper_cmds[0]
            cmd_origin = "code grepper"
        else:
            if disable_bloom:
                raise Exception(
                    "No commands found using code grepper and bloom is disabled"
                )
            else:
                cmd_text = run_bloom_query(search, debug)
                cmd_origin = "bloom"
    # print(cmd_text)
    confirm_run_of_bloom_query(
        search, cmd_text, cmd_origin, to_clipboard=not disable_clipboard
    )


if __name__ == "__main__":
    cmdline_main()
