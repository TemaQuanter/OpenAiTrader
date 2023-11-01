// This function prepares the data for submission to a local server.
function prepareFormSubmission() {
  // Capture all contenteditable elements with the class "assistant"
  const editables = document.getElementsByClassName("assistant");
  const associatedHiddenInputs = document.getElementsByClassName("hiddenInput");

  // Loop through each contenteditable element
  for (let i = 0; i < editables.length; i++) {
    // Capture content from the current contenteditable element
    const content = editables[i].innerText;

    // Set the content to the associated hidden input
    associatedHiddenInputs[i].value = content;
  } // end for
} // end function prepareFormSubmission

document
  .getElementById("dataset_form")
  .addEventListener("submit", function (e) {
    // Prevent the form's default submission
    e.preventDefault();

    // First, prepare the form data by copying contenteditable contents to hidden inputs
    prepareFormSubmission();

    // Create an object to store form data
    let formData = {
      responses: [],
    };

    // Populate formData object with form's data
    new FormData(e.target).forEach((value, key) => {
      formData.responses.push(value);
      console.log(key);
      console.log(value);
    });

    // Convert formData to JSON
    let jsonData = JSON.stringify(formData);

    // Use Fetch API to send a POST request with JSON payload
    fetch("http://0.0.0.0:9090/save_changes", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: jsonData,
    }).catch((error) => {
      console.error("Error:", error);
    });
  });
