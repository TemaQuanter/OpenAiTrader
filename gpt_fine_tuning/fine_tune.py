# This file fine-tunes ChatGPT (gpt-3.5-turbo) on a dataset.

import os
from openai import OpenAI
from dotenv import load_dotenv
import base64

# Path to the dataset to train the model on.
DATASET_PATH = "datasets/dataset_spot_improvement.jsonl"

# Import environmental variables from .env file into the module.
load_dotenv()

# Load OpenAI API key.
API_KEY = base64.b64decode(os.getenv("OPENAI_API_KEY")).decode('utf-8')

# Create openai client.
client = OpenAI(
    api_key=API_KEY,
)

# Upload the training data to the server.
file_upload_response = client.files.create(
    file=open(DATASET_PATH, "rb"),
    purpose="fine-tune",
)

# Display information about uploaded file.
print(file_upload_response)

# Get the file ID of a training file on the server.
training_file = file_upload_response.id

# Create a fine-tuning job.
fine_tuning_job_response = client.fine_tuning.jobs.create(
    training_file=training_file, model="ft:gpt-3.5-turbo-1106:artem-mikheev::8c7SQkTQ")

# Display information about fine-tuning job.
print(fine_tuning_job_response)
