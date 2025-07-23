<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog"

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

  // async function createProject(event: Event) {
  //   event.preventDefault();
  //   try {
  //     // Open a file dialog to select a folder for the new project
  //     const selected = await openDialog({
  //       directory: true,
  //       multiple: false,
  //       title: "Select Location for New Project"
  //     });
  //
  //     if (selected) {
  //       // Send the selected path to the Rust backend to open it
  //       greetMsg = await invoke("open_file", { path: selected });
  //       console.log("New project location selected:", greetMsg);
  //     }
  //   } catch (error) {
  //     console.error("Error creating project:", error);
  //     greetMsg = `Error: ${error}`;
  //   }
  // }
</script>

<main>
  <h1 class="header">Welcome to RISE</h1>

  <ul class="buttons">
    <li><button class="bt bt--new" onclick={openProject}>New Project<br>➕</button></li>
    <li><button class="bt bt--open" onclick={openProject}>Open Project <br>🗂️</button></li>
  </ul>
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
}

</style>
