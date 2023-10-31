# This file fine-tunes ChatGPT (gpt-3.5-turbo) on a dataset.

import os
import openai
from dotenv import load_dotenv
import base64

# Path to the dataset to train the model on.
DATASET_PATH = "datasets/dataset1.jsonl"

# Import environmental variables from .env file into the module.
load_dotenv()

# Load OpenAI API key.
openai.api_key = base64.b64decode(os.getenv("OPENAI_API_KEY")).decode('utf-8')

# Upload the training data to the server.
file_upload_response = openai.File.create(
    file=open(DATASET_PATH, "rb"),
    purpose='fine-tune'
)

print(file_upload_response)

# Create a fine-tuning job.
# openai.FineTuningJob.create(training_file="file-abc123", model="gpt-3.5-turbo")
