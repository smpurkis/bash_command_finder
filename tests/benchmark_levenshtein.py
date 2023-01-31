import time
import random

# from Levenshtein import distance as levenshtein_distance  # type: ignore

# time levenstein function

# Efficient Levenshtein distance function
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


def time_levenshtein_distance():
    # generate random strings
    s1 = random.choices("abcdefghijklmnopqrstuvwxyz", k=10000)
    s2 = random.choices("abcdefghijklmnopqrstuvwxyz", k=10000)
    start = time.time()
    levenshtein_distance(s1, s2)
    end = time.time()
    print(f"Time taken: {end - start}")


if __name__ == "__main__":
    time_levenshtein_distance()
