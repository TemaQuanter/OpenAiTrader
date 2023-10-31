// This function saves a new modified html file.
function saveChanges() {
  // Capture the entire modified HTML content
  const entirePageContent =
    "<!DOCTYPE html>\n" + document.documentElement.outerHTML;

  // Create a Blob from the content
  const blob = new Blob([entirePageContent], { type: "text/html" });

  // Create an Object URL for the Blob
  const url = URL.createObjectURL(blob);

  // Create a temporary anchor element for the download
  const a = document.createElement("a");
  a.href = url;
  a.download = "modified_dataset.html"; // Name of the downloaded file
  document.body.appendChild(a);

  // Trigger a click event on the anchor element to initiate the download
  a.click();

  // Clean up
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  alert(
    "Please replace the original file with the downloaded one to save changes."
  );
}
