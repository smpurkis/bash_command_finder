import subprocess as sp
import sys
from pathlib import Path
from typing import List

import pyperclip
import requests
from colorama import init, Style
from Levenshtein import distance as levenshtein_distance
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
headers = {"Authorization": os.environ["HUGGING_FACE_API_KEY"]}


def query(payload):
    response = requests.post(API_URL, headers=headers, json=payload)
    return response.json()


def load_json_cmd_examples() -> List[List[str]]:
    return json.load(open("cmd_examples.json"))


def save_json_cmd_examples(cmd_examples: List[List[str]]):
    cmd_examples = list(set(tuple([(cmd[0], cmd[1]) for cmd in cmd_examples])))
    return json.dump(
        cmd_examples, open("cmd_examples.json", "w"), sort_keys=True, indent=4
    )


EXAMPLES_CONTEXT = "Linux bash command to accomplish the task"


def form_query_base(cmd_examples: List[List[str]]) -> List[str]:
    base_query_list = [EXAMPLES_CONTEXT]
    for cmd_eg in cmd_examples:

        base_query_list.append("")
        base_query_list.append(f"# {cmd_eg[0]}")
        base_query_list.append(cmd_eg[1])
    return base_query_list


def parse_code_grepper_answer(answer: str) -> str:
    if "#" in answer:
        answer_lines = answer.split("\n")
        answer = "\n".join([a for a in answer_lines if a[0] != "#"])
    return answer


def query_code_grepper(search: str) -> str:
    api_version = 3
    query = f"bash {search}"
    base_url = f"https://www.codegrepper.com/api/get_answers_1.php?v={api_version}&s={requests.utils.quote(query)}"

    resp = requests.get(
        base_url, verify=False
    )  # verify=True errs with certificate verify failed: unable to get local issuer certificate
    resp_json = resp.json()
    answers = [parse_code_grepper_answer(a["answer"]) for a in resp_json["answers"]]
    return answers


def run_process(cmd: str, cwd: str = Path().as_posix()):
    process = sp.Popen(cmd, shell=True, stdout=sp.PIPE, cwd=cwd)
    for line in iter(process.stdout.readline, ""):
        sys.stdout.write(line.decode())
        poll = process.poll()
        if poll is not None:
            break


def parse_bloom_output(output: str, query_text: str, cmd_text: str) -> str:
    full_output = output[0]["generated_text"]
    output_split = full_output.split(query_text)
    answer_lines = []
    if len(output_split) == 2:
        answer_lines = output_split[1]
        answer_lines = answer_lines.split("\n")
    elif len(output_split) == 1:
        output_lines = output_split[0].split("\n")
        output_line_lev_dists = [
            levenshtein_distance(a, cmd_text) for a in output_lines
        ]
        last_query_line_index = output_line_lev_dists.index(min(output_line_lev_dists))
        answer_lines = output_lines[last_query_line_index + 1 :]
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
    input_text: str, cmd_text: str, to_clipboard: bool = False
) -> bool:
    sys.stdout.write(cmd_text)
    sys.stdout.write("\n")
    if to_clipboard:
        confirm_text = input("Would you like to run copy this to clipboard (y/n)? ")
    else:
        confirm_text = input("Would you like to run this cmd (y/n)? ")
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
@click.option(
    "-s",
    "--search",
    required=True,
    default="",
    help="describe in english what the terminal command does",
)
@click.option(
    "--debug",
    default=False,
    required=False,
    help="show raw query",
)
@click.option(
    "--use_cache",
    default=True,
    required=False,
    help="use the cache to reduce queries of previous searches",
)
def cmdline_main(search: str, debug: bool, use_cache: bool):
    cmd_text = None
    if use_cache:
        cmd_text = check_cache(search)
    if cmd_text is None:
        codegrepper_cmds = query_code_grepper(search)
        if len(codegrepper_cmds) == 0:
            cmd_text = run_bloom_query(search, debug)
        else:
            cmd_text = codegrepper_cmds[0]
    confirm_run_of_bloom_query(search, cmd_text, to_clipboard=True)


def run_repl():
    while True:
        print(Style.RESET_ALL)
        input_text = input("> ")
        if input_text[:2] == "# ":
            cmd_text = run_bloom_query(input_text)
            if confirm_run_of_bloom_query(input_text, cmd_text):
                run_process(cmd_text)
        else:
            run_process(input_text)


if __name__ == "__main__":
    # run_bloom_query("# print hello world to the terminal")
    # run_bloom_query("# find the first 5 .log files in the current directory")
    # run_repl()
    cmdline_main()
