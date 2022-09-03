### Bash Command Finder
TLDR: Find bash commands by describing them in plain english

This project uses [Bloom](https://huggingface.co/bigscience/bloom) model from huggingface and 
[Code grepper](https://www.codegrepper.com/) to search Bash commands from a plain english input.

See [cmd_examples.json](https://github.com/smpurkis/bash_command_finder/blob/main/cmd_examples.json) 
for examples of its usage.

Full help usage:
```
Usage: bash_command_finder.py [OPTIONS] SEARCH

  Find bash command by describing it in English.

  Please set HUGGING_FACE_API_KEY to be able to use Bloom search functionality
  E.g. "Bearer xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" from
  https://huggingface.co/bigscience/bloom

  Example usage

  - print "hello world" -> echo "hello world"

  - find all .log files in current directory -> find -name '*.log'

Options:
  --debug                show debug logging, i.e. raw query sent to bloom
  --disable_cache        use the cache to reduce queries of previous searches
  --disable_bloom        disable bloom search
  --disable_codegrepper  disable code grepper search
  --disable_clipboard    disable asking to copy to clipboard, will only print
                         the suggestions
  --help                 Show this message and exit.
```

Example usages:

Sometimes Bloom gives the best results, but the API is rate limited, so the default it to search code grepper first.
```
> python bash_command_finder.py "list all files and metadata in current directory" --disable_codegrepper
ls -l
Would you like to run copy this to clipboard (y/n)? y
```


```
> python bash_command_finder.py "remove directory" --disable_bloom
From code grepper: rm -rf directory-name
Would you like to copy this command to clipboard (y/n)? y
```