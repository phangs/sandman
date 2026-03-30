# Frontend Tasks

## UI Components
- [ ] **First-Time Setup & Project Switcher:** A modal/dropdown to select or input the absolute path of a local project directory.
- [ ] **IDE Layout Shell:** Implement `react-resizable-panels` to create the Activity Bar, Side Bar, Editor Group, and Bottom Panel.
- [ ] **Code Diff viewer / Editor:** Integrate `@monaco-editor/react` in the central Editor Group to show AI code changes and file contents.
- [ ] **Terminal Panel:** Integrate `xterm.js` in the Bottom Panel connected to Tauri IPC events (`listen("log")`) to stream real-time Agent logs.
- [ ] **Side Bar / Explorer:** Build a VS Code style file tree component and a collapsed Kanban Story list component.

## State Management
- [ ] React Query / SWR mapped to Tauri Commands instead of HTTP fetches for story updates.
- [ ] Tauri IPC listeners for real-time status transitions (e.g., card moving automatically).