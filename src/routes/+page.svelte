<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog"

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
  <div class="main-container">
  <div>
  <ul class="buttons">
    <li><button class="bt bt--new" onclick={createProject}>New Project<br>➕</button></li>
    <li><button class="bt bt--open" onclick={openProject}>Open Project <br>🗂️</button></li>
  </ul>
  </div>
  <div>
    <h2>Recent Projects</h2>
    <ul class="buttons">
<!--      TODO: create cycle to go through recent projects -->
    </ul>
  </div>
  </div>

  {#if showProjectNameDialog}
    <div class="dialog-overlay">
      <div class="dialog">
        <h2>Create New Project</h2>
        <div class="form-group">
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
  @use 'main.scss';
</style>