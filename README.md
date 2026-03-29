# Sandman - Autonomous AI Software Factory

Sandman is a monorepo project structured as an "Autonomous AI Software Factory". It leverages modern web technologies and npm workspaces to manage its various applications and packages seamlessly.

## Architecture

This project is structured as an npm monorepo with the following main applications and packages:

- `apps/web`: The front-end client interface.
- `apps/server`: The backend API service.
- `packages/runner`: Core logic and execution runner.

## Getting Started

### Prerequisites

Ensure you have [Node.js](https://nodejs.org/) installed along with `npm`.

### Installation

To install all dependencies for the entire project from the root folder, run:

```bash
npm run install:all
```

Alternatively, standard npm install will also run the workspace install:

```bash
npm install
```

### Running the Environment Locally

You can run individual parts of the application or the entire stack simultaneously using `concurrently`.

To start all services (`web`, `server`, and `runner`) at once:

```bash
npm run dev
```

#### Running Individual Services

- **Web Frontend:**
  ```bash
  npm run dev:web
  ```

- **Backend Server:**
  ```bash
  npm run dev:server
  ```

- **Runner Package:**
  ```bash
  npm run dev:runner
  ```

## Repository Structure

```
sandman/
├── apps/         # Applications (web interface, backend server, etc.)
│   ├── web/
│   └── server/
├── packages/     # Shared packages and libraries
│   └── runner/
├── docs/         # Documentation resources
├── package.json  # Root package file containing main scripts and workspaces schema
└── README.md     # This readme file
```

## Workspaces Setup

This repository relies on [npm workspaces](https://docs.npmjs.com/cli/v10/using-npm/workspaces) defined in the root `package.json`:

```json
"workspaces": [
  "apps/*",
  "packages/*"
]
```

## Contributing

Make sure to install dependencies and test changes locally using the workspace scripts before opening a pull request.
