<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { open } from "@tauri-apps/api/dialog"
  import { onMount } from 'svelte';

  let showProjectNameDialog = false;
  let projectName = "rise-project";
  let selectedBasePath = "";
  let projectNameInput: HTMLInputElement;
  let sanitizedName = "";
  let showSanitizeWarning = false;

  async function openProject(event: Event) {
    event.preventDefault();
    try {
      const selectedPath = await open({
        multiple: false,
        directory: true,
        title: 'Open Project Folder'
      })

      if (selectedPath) {
        await invoke("open_project", { path: selectedPath });
        localStorage.setItem('projectPath', selectedPath as string);
        window.location.href = "/editor";
      }
    } catch (error) {
      console.error("Error opening project:", error);
    }
  }

  async function createProject(event: Event) {
    event.preventDefault();
    const selectedPath = await open({
      multiple: false,
      directory: true,
      title: 'Select Base Folder'
    })

    if (selectedPath) {
      selectedBasePath = selectedPath as string;
      projectName = "rise-project"; // Reset to default
      sanitizedName = "rise-project"; // Initialize sanitized name
      showSanitizeWarning = false; // Reset warning flag
      showProjectNameDialog = true;

      // Focus the input field after the dialog is shown
      setTimeout(() => {
        if (projectNameInput) {
          projectNameInput.focus();
          projectNameInput.select();
        }
      }, 100);
    }
  }

  async function confirmProjectCreation() {
    if (selectedBasePath) {
      try {
        // Make sure sanitized name is up to date
        handleProjectNameChange();

        const path = await invoke("create_project", { 
          path: selectedBasePath,
          projectName: sanitizedName // Use the sanitized name
        });
        localStorage.setItem('projectPath', path as string);
        showProjectNameDialog = false;
        window.location.href = "/editor";
      } catch (error) {
        console.error("Error creating project:", error);
        alert(`Error creating project: ${error}`);
      }
    }
  }

  // Function to sanitize project name (same logic as backend)
  function sanitizeProjectName(name: string): string {
    const sanitized = name
      .split('')
      .map(c => /[a-zA-Z0-9\-_ ]/.test(c) ? c : '_')
      .join('')
      .trim();

    return sanitized || "rise-project";
  }

  // Update sanitized name whenever project name changes
  function handleProjectNameChange() {
    const newSanitized = sanitizeProjectName(projectName);
    sanitizedName = newSanitized;
    showSanitizeWarning = newSanitized !== projectName && projectName.trim() !== '';
  }

  function cancelProjectCreation() {
    showProjectNameDialog = false;
    selectedBasePath = "";
    projectName = "rise-project";
    sanitizedName = "rise-project";
    showSanitizeWarning = false;
  }
</script>

<main>
  <h1 class="header">Welcome to RISE</h1>

  <ul class="buttons">
    <li><button class="bt bt--new" onclick={createProject}>New Project<br>➕</button></li>
    <li><button class="bt bt--open" onclick={openProject}>Open Project <br>🗂️</button></li>
  </ul>

  {#if showProjectNameDialog}
    <div class="dialog-overlay">
      <div class="dialog">
        <h2>Create New Project</h2>
        <p>Enter a name for your project or use the default name.</p>
        <div class="form-group">
          <label for="project-name">Project Name:</label>
          <input 
            type="text" 
            id="project-name" 
            bind:value={projectName} 
            bind:this={projectNameInput}
            placeholder="Enter project name"
            oninput={handleProjectNameChange}
            onkeydown={(e) => e.key === 'Enter' && confirmProjectCreation()}
          />
          {#if showSanitizeWarning}
            <div class="warning-message">
              <p>Your project name contains invalid characters that will be replaced.</p>
              <p>It will be saved as: <strong>{sanitizedName}</strong></p>
            </div>
          {/if}
        </div>
        <div class="dialog-buttons">
          <button class="dialog-button cancel" onclick={cancelProjectCreation}>Cancel</button>
          <button class="dialog-button confirm" onclick={confirmProjectCreation}>Create Project</button>
        </div>
      </div>
    </div>
  {/if}
</main>

<style lang="scss">
:global(body) {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;
  color: var(--accent-dark);
  background-color: var(--white);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}

.dialog {
  background-color: var(--white);
  border-radius: 8px;
  padding: 2rem;
  width: 90%;
  max-width: 500px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}

.dialog h2 {
  margin-top: 0;
  color: var(--accent-dark);
}

.form-group {
  margin-bottom: 1.5rem;
}

.form-group label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
}

.form-group input {
  width: 100%;
  padding: 0.75rem;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 1rem;
  font-family: inherit;
}

.warning-message {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background-color: #fff3cd;
  border: 1px solid #ffeeba;
  border-radius: 4px;
  color: #856404;
  font-size: 0.9rem;
}

.warning-message p {
  margin: 0.25rem 0;
}

.warning-message strong {
  font-weight: 600;
}

.dialog-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
}

.dialog-button {
  padding: 0.6em 1.2em;
  border-radius: 8px;
  border: none;
  font-size: 1rem;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 0.15s ease-in-out;

  &.cancel {
    background-color: #e0e0e0;
    color: #333;

    &:hover {
      background-color: #d0d0d0;
    }
  }

  &.confirm {
    background-color: var(--blue);
    color: black;
    box-shadow: 1px 1px 1px 1px var(--accent-green);

    &:hover {
      background-color: var(--accent-green);
    }
  }
}

main {
  margin: 0;
  padding: 0;
  height: 100vh;
  width: 100vw;
  display: flex;
  flex-direction: column;
  text-align: center;
  overflow: hidden;
}

.header {
  margin-top: 10vh;
  font-size: xxx-large;
}

.buttons {
  list-style: none;
  margin-top: 30vh;
  margin-left: 10%;
}

li {
  max-width: 240px;
}

.bt {
  margin: 10px;
  border-radius: 8px;
  border: none;
  padding: 0.6em 1.2em;
  font-size: 1.8em;
  font-weight: 500;
  font-family: inherit;
  color: black;
  box-shadow: 2px 2px 1px 1px var(--accent-green);
  background-color: var(--blue);
  cursor: pointer;

  &:hover {
    background-color: var(--accent-green);
    transition: background-color 0.15s ease-in-out;
  }
}

@media(max-width: 900px) or (max-height:900px) {
  .header {
    text-align: center;
    margin-top: 15vh;
    font-size: xxx-large;
  }

  .buttons {
    margin-top: 20vh;
  }

  .bt {
    font-size: 1.3em;
  }

  li {
    max-width: 180px;
  }
}

@media (max-width: 700px) or (max-height: 650px) {
  .header {
    text-align: center;
    margin-top: 20vh;
    font-size: xx-large;
  }

  .buttons {
    margin-top: 15vh;
  }

  .bt {
    font-size: 1em;
  }

  li {
    width: 100%;
    max-width: 140px;
  }
}

@media (prefers-color-scheme: dark) {
  :global(body) {
    color: #f6f6f6;
    background-color: var(--background-dark);
  }

  a:hover {
    color: #24c8db;
  }

  .bt {
    //color: #ffffff;
    //background-color: #0f0f0f98;

    &--open {
      //background-color: #616161;
    }
    &--new {
      //background-color: #e0f7fa;
    }
  }

  .dialog {
    background-color: var(--background-dark);
    color: #f6f6f6;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  .dialog h2 {
    color: #f6f6f6;
  }

  .form-group input {
    background-color: #333;
    border-color: #555;
    color: #f6f6f6;
  }

  .warning-message {
    background-color: #332701;
    border-color: #664d03;
    color: #ffda6a;
  }

  .dialog-button {
    &.cancel {
      background-color: #444;
      color: #f6f6f6;

      &:hover {
        background-color: #555;
      }
    }

    &.confirm {
      background-color: var(--blue);
      color: #f6f6f6;

      &:hover {
        background-color: var(--accent-green);
      }
    }
  }
}

</style>
