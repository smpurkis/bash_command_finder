/*
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
    return json.load(open("src/cmd_examples.json"))


def save_json_cmd_examples(cmd_examples: List[List[str]]):
    cmd_examples = []
    for cmd in cmd_examples:
        cmd_examples.append([cmd[0], cmd[1]])

    cmd_examples_set = set(cmd_examples)
    cmd_examples = list(cmd_examples_set)
    json.dump(
        cmd_examples, open("src/cmd_examples.json", "w"), sort_keys=True, indent=4
    )


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
    # confirm_run_of_bloom_query(
    #     search, cmd_text, cmd_origin, to_clipboard=not disable_clipboard
    # )


if __name__ == "__main__":
    cmdline_main()
*/

// convert to rust

use std::fs::File;
use std::io::prelude::*;

extern crate clipboard;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use reqwest::blocking::Client;

use std::collections::HashMap;

const API_URL: &str = "https://api-inference.huggingface.co/models/bigscience/bloom";
const EXAMPLES_CONTEXT: &str = "Linux bash command to accomplish the task";

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    let mut d = vec![vec![0; len2 + 1]; len1 + 1];
    for i in 0..=len1 {
        d[i][0] = i;
    }
    for j in 0..=len2 {
        d[0][j] = j;
    }
    for j in 1..=len2 {
        for i in 1..=len1 {
            if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) {
                d[i][j] = d[i - 1][j - 1];
            } else {
                d[i][j] = std::cmp::min(
                    std::cmp::min(d[i - 1][j] + 1, d[i][j - 1] + 1),
                    d[i - 1][j - 1] + 1,
                );
            }
        }
    }
    d[len1][len2]
}

fn load_json_cmd_examples() -> Vec<Vec<String>> {
    let mut file = File::open("cmd_examples.json").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let cmd_examples: Vec<Vec<String>> = serde_json::from_str(&contents).unwrap();
    cmd_examples
}

fn save_json_cmd_examples(cmd_examples: Vec<Vec<String>>) {
    let cmd_examples_json = serde_json::to_string(&cmd_examples).unwrap();
    let mut file = File::create("cmd_examples.json").unwrap();
    file.write_all(cmd_examples_json.as_bytes()).unwrap();
}

fn form_query_base(cmd_examples: Vec<Vec<String>>) -> Vec<String> {
    let mut base_query_list = Vec::new();
    for cmd_eg in cmd_examples {
        base_query_list.push(format!("# {}", cmd_eg[0]));
        base_query_list.push(format!("> {}", cmd_eg[1]));
    }
    base_query_list
}

fn query_code_grepper(text: &str, debug: bool) -> Vec<String> {
    let client = Client::new();
    // base_url = f"https://www.codegrepper.com/api/get_answers_1.php?v={api_version}&s={requests.utils.quote(query)}"
    let url = format!(
        "https://www.codegrepper.com/api/get_answers_1.php?v=3&s={}",
        text
    );
    // with verify false
    let resp = client.get(&url).send().unwrap();
    let resp_text = resp.text().unwrap();
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text).unwrap();
    let codegrepper_cmds: Vec<String> = resp_json
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["answer"].as_str().unwrap().to_string())
        .collect();
    if debug {
        println!("codegrepper cmds: {:?}", codegrepper_cmds);
    }
    codegrepper_cmds
}

fn correct_answer_line(answer_line: &str) -> String {
    let answer_comps: Vec<&str> = answer_line.split(" ").collect();
    let first_cmd = answer_comps[0];
    let first_cmd = first_cmd
        .chars()
        .filter(|x| x.is_alphanumeric())
        .collect::<String>();
    let mut answer_comps = answer_comps;
    answer_comps[0] = &first_cmd;
    let corrected_answer_line = answer_comps.join(" ");
    corrected_answer_line
}

fn parse_bloom_response(resp_text: String, query_text: String, cmd_text: String) -> String {
    let full_output = resp_text;
    let output_split = full_output.split(&query_text).collect::<Vec<&str>>();
    let mut answer_lines: Vec<String> = vec![];
    if output_split.len() == 2 {
        answer_lines = output_split[1]
            .split("\n")
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
    } else if output_split.len() == 1 {
        let output_lines = output_split[0].split("\n").collect::<Vec<&str>>();
        let output_line_lev_dists = output_lines
            .iter()
            .map(|x| levenshtein_distance(x, &cmd_text))
            .collect::<Vec<usize>>();
        let last_query_line_index = output_line_lev_dists
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .unwrap()
            .0;
        answer_lines = output_lines[last_query_line_index + 1..]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
    }
    if answer_lines.len() == 0 {
        panic!("No command via Bloom could be found!");
    }
    let answer_line = answer_lines
        .iter()
        .filter(|x| x != &"")
        .map(|x| x.to_string())
        .collect::<Vec<String>>()[0]
        .to_string();
    let answer_line = correct_answer_line(&answer_line);
    answer_line
}

fn query(payload: HashMap<String, String>) -> String {
    let client = Client::new();
    let resp = client.post(API_URL).json(&payload).send().unwrap();
    let resp_text = resp.text().unwrap();
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text).unwrap();
    let result = resp_json[0]["generated_text"].as_str().unwrap();
    result.to_string()
}

fn run_bloom_query(text: String, debug: bool) -> String {
    let cmd_examples = load_json_cmd_examples();
    let mut base_query_list = form_query_base(cmd_examples);
    base_query_list.push(text.clone());
    let base_query = base_query_list.join("\n");
    let mut payload = HashMap::new();
    payload.insert("inputs".to_string(), base_query.clone());
    let output = query(payload);
    if debug {
        println!("bloom result: {}", output);
    }
    let answer_line = parse_bloom_response(output, base_query, text);
    answer_line
}

fn confirm_run_of_bloom_query(
    input_text: String,
    cmd_text: String,
    cmd_origin: String,
    to_clipboard: bool,
) -> bool {
    println!("From {}: {}", cmd_origin, cmd_text);
    let mut confirm_text = String::new();
    if to_clipboard {
        print!("Would you like to copy this command to clipboard (y/n)? ");
    } else {
        // println!("");
        print!("Is this command correct (y/n)? ");
    }
    std::io::stdin().read_line(&mut confirm_text).unwrap();
    if ["y", "yes"].contains(&confirm_text.trim().to_lowercase().as_str()) {
        let mut cmd_examples = load_json_cmd_examples();
        cmd_examples.push(vec![input_text, cmd_text.clone()]);
        save_json_cmd_examples(cmd_examples);
        if to_clipboard {
            set_clipboard(&cmd_text);
        }
        true
    } else {
        false
    }
}

fn set_clipboard(text: &str) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    ctx.set_contents(text.to_string()).unwrap();
}

fn check_cache(text: &str, debug: bool) -> Option<String> {
    let cmd_examples = load_json_cmd_examples();
    for cmd_eg in cmd_examples {
        if cmd_eg[0] == text {
            if debug {
                println!("cache hit: {}", cmd_eg[1]);
            }
            return Some(cmd_eg[1].clone());
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut search = String::new();
    let mut debug = false;
    let mut disable_cache = false;
    let mut disable_codegrepper = false;
    let mut disable_bloom = false;
    let mut disable_clipboard = false;
    for arg in args {
        if arg == "--debug" {
            debug = true;
        } else if arg == "--disable-cache" {
            disable_cache = true;
        } else if arg == "--disable-codegrepper" {
            disable_codegrepper = true;
        } else if arg == "--disable-bloom" {
            disable_bloom = true;
        } else if arg == "--disable-clipboard" {
            disable_clipboard = true;
        } else {
            search = arg;
        }
    }
    if search.is_empty() {
        // error
        panic!("No search term provided");
        return;
    }
    let mut cmd_origin = "cache";
    let mut cmd_text = None;
    if !disable_cache {
        cmd_text = check_cache(&search, debug);
    }
    if cmd_text.is_none() {
        let mut codegrepper_cmds: Vec<String> = Vec::new();
        if !disable_codegrepper {
            codegrepper_cmds = query_code_grepper(&search, debug);
        };
        if codegrepper_cmds.len() > 1 {
            cmd_text = Some(codegrepper_cmds[0].clone());
            cmd_origin = "code grepper";
        } else if disable_bloom {
            panic!("No commands found using code grepper and bloom is disabled");
        } else {
            cmd_text = Some(run_bloom_query(search.clone(), debug));
            cmd_origin = "bloom";
        }
    }
    // println!("{cmd_text:?}");
    confirm_run_of_bloom_query(
        search,
        cmd_text.unwrap(),
        cmd_origin.to_string(),
        !disable_clipboard,
    );
}
