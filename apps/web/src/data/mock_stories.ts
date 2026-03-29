export type StoryState = 'Draft' | 'Ready' | 'Processing' | 'Failed' | 'Success';

export interface Story {
  id: string;
  title: string;
  description: string;
  state: StoryState;
  isReadyForAI: boolean;
  acceptanceCriteria?: string[];
  logs?: string[];
}

export const MOCK_STORIES: Story[] = [
  {
    id: "ST-001",
    title: "Implement User Authentication",
    description: "Create a login and signup page with JWT authentication.",
    state: "Draft",
    isReadyForAI: false,
    acceptanceCriteria: [
      "User can sign up with email and password",
      "User can login and receive a JWT token",
      "Protected routes only accessible with token"
    ]
  },
  {
    id: "ST-002",
    title: "Setup Redis Connection",
    description: "Integrate Redis for real-time task queuing and status updates.",
    state: "Ready",
    isReadyForAI: true,
  },
  {
    id: "ST-003",
    title: "Design Kanban Card",
    description: "Create a reusable Kanban card component with visual state cues.",
    state: "Processing",
    isReadyForAI: true,
    logs: [
      "Starting Story Agent...",
      "Analyzing requirement...",
      "Generating code scaffold..."
    ]
  },
  {
    id: "ST-004",
    title: "Implement API Gateway",
    description: "Proxy requests from frontend to microservices.",
    state: "Failed",
    isReadyForAI: true,
    logs: [
      "Runner starting...",
      "ERROR: Port 8080 already in use",
      "Retrying 1/3..."
    ]
  },
  {
    id: "ST-005",
    title: "Database Migration Script",
    description: "Automate SQL migrations for the main database schema.",
    state: "Success",
    isReadyForAI: true,
    acceptanceCriteria: [
      "Script runs without errors",
      "Schema updated to v1.2",
      "Rollback script verified"
    ]
  }
];
