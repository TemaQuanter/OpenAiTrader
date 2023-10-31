# Pricing and default n_epochs estimate
from typing import List, Dict


def estimate_costs(dataset: List[Dict], convo_lens, TARGET_EPOCHS, MIN_TARGET_EXAMPLES, MAX_DEFAULT_EPOCHS, MAX_TARGET_EXAMPLES, MIN_DEFAULT_EPOCHS, MAX_TOKENS_PER_EXAMPLE) -> None:
    """This function displays the information about approximate costs of fine-tuning a model
    on a particular dataset."""
    n_epochs = TARGET_EPOCHS
    n_train_examples = len(dataset)
    if n_train_examples * TARGET_EPOCHS < MIN_TARGET_EXAMPLES:
        n_epochs = min(MAX_DEFAULT_EPOCHS,
                       MIN_TARGET_EXAMPLES // n_train_examples)
    elif n_train_examples * TARGET_EPOCHS > MAX_TARGET_EXAMPLES:
        n_epochs = max(MIN_DEFAULT_EPOCHS,
                       MAX_TARGET_EXAMPLES // n_train_examples)

    n_billing_tokens_in_dataset = sum(
        min(MAX_TOKENS_PER_EXAMPLE, length) for length in convo_lens)
    print(
        f"Dataset has ~{n_billing_tokens_in_dataset} tokens that will be charged for during training")
    print(f"By default, you'll train for {n_epochs} epochs on this dataset")
    print(
        f"By default, you'll be charged for ~{n_epochs * n_billing_tokens_in_dataset} tokens")
