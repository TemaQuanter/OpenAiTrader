// This function prepares the data for submission to a local server.
function prepareFormSubmission() {
  // Capture all contenteditable elements with the class "user"
  const editables_user = document.getElementsByClassName("user");
  const associatedHiddenInputsRequests =
    document.getElementsByClassName("hiddenInputRequest");

  // Capture all contenteditable elements with the class "assistant"
  const editables_assistant = document.getElementsByClassName("assistant");
  const associatedHiddenInputsResponses = document.getElementsByClassName(
    "hiddenInputResponse"
  );

  // Loop through each contenteditable element
  for (let i = 0; i < editables_assistant.length; i++) {
    // Capture content from the current contenteditable element
    const content_request = editables_user[i].innerText;
    const content_response = editables_assistant[i].innerText;

    // Set the content to the associated hidden input
    associatedHiddenInputsRequests[i].value = content_request;
    associatedHiddenInputsResponses[i].value = content_response;
  } // end for
} // end function prepareFormSubmission

function createMiniForms(dataset) {
  const container = document.getElementById("dataset_form"); // ID of the container where you want to append the forms

  for (let i = 0; i < dataset["requests"].length; i++) {
    // Create elements
    const blockDiv = document.createElement("div");
    blockDiv.className = "block";
    blockDiv.id = `block_${i + 1}`;

    const titleP = document.createElement("p");
    titleP.className = "block_title";
    titleP.innerHTML = `Example #${i + 1}`;

    const requestP = document.createElement("p");
    requestP.className = "request";
    requestP.innerHTML = "Request";

    const userPre = document.createElement("pre");
    userPre.className = "user";
    userPre.contentEditable = "true";
    userPre.innerHTML = dataset["requests"][i];

    const hiddenInputRequest = document.createElement("input");
    hiddenInputRequest.type = "hidden";
    hiddenInputRequest.className = "hiddenInputRequest";
    hiddenInputRequest.name = `request_${i + 1}`;

    const responseP = document.createElement("p");
    responseP.className = "response";
    responseP.innerHTML = "Response";

    const assistantPre = document.createElement("pre");
    assistantPre.className = "assistant";
    assistantPre.contentEditable = "true";
    assistantPre.innerHTML = dataset["requests"][i];

    const hiddenInputResponse = document.createElement("input");
    hiddenInputResponse.type = "hidden";
    hiddenInputResponse.className = "hiddenInputResponse";
    hiddenInputResponse.name = `response_${i + 1}`;

    const saveButton = document.createElement("button");
    saveButton.type = "submit";
    saveButton.innerHTML = "Save All changes";

    const deleteButton = document.createElement("button");
    deleteButton.type = "button";
    deleteButton.innerHTML = "Remove element";

    deleteButton.style.visibility = "hidden";

    // Add an event listener to the button
    deleteButton.addEventListener("click", function () {
      // Select the element to remove.
      const elementToRemove = this.parentElement;

      // Remove the element
      if (elementToRemove) {
        elementToRemove.remove();
      }
    });

    // Append elements
    blockDiv.appendChild(titleP);
    blockDiv.appendChild(requestP);
    blockDiv.appendChild(userPre);
    blockDiv.appendChild(responseP);
    blockDiv.appendChild(assistantPre);
    blockDiv.appendChild(hiddenInputRequest);
    blockDiv.appendChild(hiddenInputResponse);
    blockDiv.appendChild(saveButton);
    blockDiv.appendChild(deleteButton);

    container.appendChild(blockDiv);
  }
}

function fetchData() {
  fetch("http://0.0.0.0:9090/get_data")
    .then((response) => {
      if (!response.ok) {
        throw new Error("Network response was not ok");
      }
      return response.json();
    })
    .then((data) => {
      // Create the blocks with training examples.
      createMiniForms(data);

      // Use the fetched data here, e.g., update the DOM, store in a variable, etc.

      // Get all user requests.
      const userRequests = document.getElementsByClassName("user");

      // Get all assistant replies.
      const assistantReplies = document.getElementsByClassName("assistant");

      // Traverse each assistant reply and replace it with the most
      // recent one.

      for (let i = 0; i < assistantReplies.length; i++) {
        // Replace the content.
        userRequests[i].innerHTML = data.requests[i];
        assistantReplies[i].innerHTML = data.responses[i];

        // console.log(data.responses[i]);
      } // end for

      //   console.log(data);
    })
    .catch((error) => {
      console.log(
        "There was a problem with the fetch operation:",
        error.message
      );
    });
}
