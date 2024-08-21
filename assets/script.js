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
      }).then(() => {
        const checkedItems = document.getElementById("checked-items");
        const uncheckedItems = document.getElementById("unchecked-items");

        const listElement = checkbox.parentNode;

        if (state) {
          uncheckedItems.removeChild(listElement);
          checkedItems.appendChild(listElement);
        } else {
          checkedItems.removeChild(listElement);
          uncheckedItems.appendChild(listElement);
        }
      });
    };
  }
};

document.addEventListener("DOMContentLoaded", onDocumentLoad);
