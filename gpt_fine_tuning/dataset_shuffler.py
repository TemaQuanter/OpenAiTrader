# This file contains a script that shuffles dataset to prevent
# bias in fine-tuning process.

import random

DATASET_PATH = "datasets/dataset.jsonl"
SHUFFLED_DATASET_PATH = "datasets/shuffled_dataset.jsonl"


# Store the dataset in an array.
dataset = []

# Open a file with the dataset.
with open(DATASET_PATH, "r") as f:
    # Read all lines from the dataset consequently.
    while True:
        # Read a line from the dataset.
        line = f.readline()

        # Check that that line is not the end of file.
        if line == "":
            # The line is the end of file
            break

        # Add the line to the dataset array.
        dataset.append(line)


# Shuffle the array with fine-tuning samples.
random.shuffle(dataset)


# Create a new file with a shuffled dataset.
with open(SHUFFLED_DATASET_PATH, "w") as f:
    # Write all samples in a shuffled order to the file.
    for line in dataset:
        f.write(line)