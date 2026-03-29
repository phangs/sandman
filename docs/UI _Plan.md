
---

## ### 1. The Kanban Board Layout
The UI needs to be more than just columns; it needs to be a **Command Center**.

* **Header:** Project Name, Connection Status (to Backend/Redis), and a "Global Settings" (LLM Provider mapping).
* **The Board:** 5-6 Columns. Each card represents a `Story`.
* **The Card:** * Title & ID.
    * **Status Indicators:** (e.g., "AI Processing", "Retrying 1/3", "Success").
    * **The "Ready for AI" Toggle:** A high-visibility switch.
* **Side Panel (The "Terminal"):** A slide-out panel that opens when you click a card, showing the live streaming logs from the Local Runner.



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

## ### 4. UI Tech Stack Recommendations (Zero-Cost)
* **Styling:** **Tailwind CSS** (Fastest for prototyping).
* **Icons:** **Lucide React** (Clean, developer-focused icons).
* **Components:** **Shadcn/UI** (Pre-built accessible components like Switches, Dialogs, and Cards).



---

