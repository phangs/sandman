
---

## ### 1. The "Web IDE" Layout
Instead of a simple dashboard, Sandman will look exactly like VS Code, using a dark theme (`VS Code Dark+`) and customizable docking panels.

* **Activity Bar (Far Left):** Thin vertical bar for switching main views (e.g., Kanban/Stories, File Explorer, System Settings).
* **Side Bar (Left):** 
    * If "Kanban" is selected: Shows a compacted list of active stories.
    * If "Explorer" is selected: Shows the target project's file tree (fetching from local runner).
* **Editor Group (Center):** Powered by **Monaco Editor**. 
    * Displays the `sandman.config.json` file.
    * Shows side-by-side **Code Diffs** when the Reviewer Agent proposes changes.
    * Shows a full-width Kanban Board view if no files are open.
* **Panel (Bottom):** Powered by **Xterm.js**. A real-time terminal streaming `stdout`/`stderr` from the Local Runner and AI agent thoughts over WebSocket.



---

## ### 2. The Frontend Task List (Step-by-Step)

To get moving without Docker, here is how we scaffold the UI:

### **Phase 1: The Skeleton**
- [ ] **Scaffold:** Initialize Vite + React + Tailwind.
- [ ] **Layout:** Create a responsive grid for the 5 columns (Backlog, To Do, In Progress, Review, Done).
- [ ] **Data Mocking:** Create a `mock_stories.json` to test the visual state of cards without a database.

### **Phase 2: Interaction**
- [ ] **Drag-and-Drop:** Implement `dnd-kit` to allow manual moving of cards (this should eventually update the DB status).
- [ ] **The Toggle:** Add the "Ready for AI" switch to the Backlog cards.
- [ ] **Modals:** Create a "Story Detail" view where you can edit the raw requirement or see the AI-generated Acceptance Criteria.

### **Phase 3: Real-time Plumbing**
- [ ] **Socket.io Integration:** Setup a listener to receive "Logs" from the backend.
- [ ] **Visual Cues:** Add loading spinners and "Pulse" effects to cards currently being handled by an agent.

---

## ### 3. Defining the "Story Card" States
Since the AI moves the cards, the UI needs clear visual states:

| State | Visual Cue | Meaning |
| :--- | :--- | :--- |
| **Draft** | Gray Border | User is still typing the requirement. |
| **Ready** | Blue Glow | "Ready for AI" is ON; waiting for Story Agent. |
| **Processing** | Animated Pulse | Builder Agent is currently writing code locally. |
| **Failed** | Red Border | Build/Test failed. Shows "Retry" button. |
| **Success** | Green Check | Code passed review and testing. |

---

## ### 4. UI Tech Stack Recommendations (Zero-Cost & Professional IDE)
* **Styling:** **Tailwind CSS** (Configured accurately to `VS Code Dark+` colors).
* **Icons:** **Lucide React** & VS Code standard File Icons.
* **Layout Engine:** **`react-resizable-panels`** (for standard VS Code draggable split panes).
* **Code Editor:** **`@monaco-editor/react`** (The exact engine powering VS Code).
* **Terminal:** **`xterm.js`** + `@xterm/addon-fit` (The exact terminal powering VS Code).
* **Components:** **Shadcn/UI** (for dropdowns, menus, setup modals).



---

