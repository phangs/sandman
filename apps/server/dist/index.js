"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const express_1 = __importDefault(require("express"));
const http_1 = __importDefault(require("http"));
const socket_io_1 = require("socket.io");
const cors_1 = __importDefault(require("cors"));
const app = (0, express_1.default)();
const server = http_1.default.createServer(app);
const io = new socket_io_1.Server(server, {
    cors: {
        origin: "*",
        methods: ["GET", "POST"]
    }
});
const PORT = process.env.PORT || 3001;
app.use((0, cors_1.default)());
app.use(express_1.default.json());
let stories = [
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
// Routes
app.get('/api/stories', (req, res) => {
    res.json(stories);
});
app.post('/api/stories', (req, res) => {
    const newStory = {
        ...req.body,
        id: `ST-${(stories.length + 1).toString().padStart(3, '0')}`,
        state: req.body.state || 'Draft',
        isReadyForAI: req.body.isReadyForAI || false,
        logs: []
    };
    stories.push(newStory);
    res.status(201).json(newStory);
});
app.put('/api/stories/:id', (req, res) => {
    const { id } = req.params;
    const index = stories.findIndex(s => s.id === id);
    if (index !== -1) {
        stories[index] = { ...stories[index], ...req.body };
        res.json(stories[index]);
        // Notify clients about the update
        io.emit('storyUpdated', stories[index]);
    }
    else {
        res.status(404).json({ message: "Story not found" });
    }
});
// Socket.io for real-time logs
io.on('connection', (socket) => {
    console.log('A client connected');
    socket.on('disconnect', () => {
        console.log('Client disconnected');
    });
    // Example: Listen for log events from runners (simulated for now)
    socket.on('addLog', ({ storyId, log }) => {
        const story = stories.find(s => s.id === storyId);
        if (story) {
            if (!story.logs)
                story.logs = [];
            story.logs.push(log);
            io.emit('logAdded', { storyId, log });
        }
    });
});
server.listen(PORT, () => {
    console.log(`Server is running on port ${PORT}`);
});
