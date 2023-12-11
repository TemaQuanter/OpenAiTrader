# This file fine-tunes ChatGPT (gpt-3.5-turbo) on a dataset.

import os
import openai
from dotenv import load_dotenv
import base64

# Path to the dataset to train the model on.
DATASET_PATH = "datasets/dataset_calibration.jsonl"

# Import environmental variables from .env file into the module.
load_dotenv()

# Load OpenAI API key.
openai.api_key = base64.b64decode(os.getenv("OPENAI_API_KEY")).decode('utf-8')

# Upload the training data to the server.
file_upload_response = openai.File.create(
    file=open(DATASET_PATH, "rb"),
    purpose="fine-tune",
    user_provided_filename="dataset-short-long-calibration"
)

# Display information about uploaded file.
print(file_upload_response)

# Get the file ID of a training file on the server.
training_file = file_upload_response["id"]

# Create a fine-tuning job.
fine_tuning_job_response = openai.FineTuningJob.create(
    training_file=training_file, model="ft:gpt-3.5-turbo-0613:artem-mikheev::8RN67eso")

# Display information about fine-tuning job.
print(fine_tuning_job_response)
