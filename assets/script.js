const onDocumentLoad = () => {
  const checkboxes = document.querySelectorAll('input[type="checkbox"]');

  for (const checkbox of checkboxes) {
    checkbox.onclick = async () => {
      const state = checkbox.checked;

      await fetch(`/update/${checkbox.id}`, {
        method: "PATCH",
        body: JSON.stringify({ state }),
        headers: {
          "Content-Type": "application/json",
        },
      });
    };
  }
};

document.addEventListener("DOMContentLoaded", onDocumentLoad);
