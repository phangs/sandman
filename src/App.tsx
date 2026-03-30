import { useState } from 'react';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { Folder, ListTodo, Settings } from 'lucide-react';
import { KanbanBoard } from './components/KanbanBoard';
import { TerminalPanel } from './components/TerminalPanel';
import { FileExplorer } from './components/FileExplorer';
import { SettingsView } from './components/SettingsView';
import { SidebarChat } from './components/SidebarChat';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [activeTab, setActiveTab] = useState<'kanban' | 'explorer' | 'settings'>('kanban');
  const [activeProject, setActiveProject] = useState<string | null>(null);

  const handleSelectProject = async () => {
    console.log("Starting project selection...");
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Sandman Project Workspace'
      });
      
      console.log("Selected result:", selected);
      
      if (selected) {
        const path = selected as string;
        console.log("Invoking switch_project for:", path);
        await invoke('switch_project', { path });
        console.log("Project switch successful, updating state...");
        setActiveProject(path);
        setActiveTab('kanban');
      }
    } catch (err) {
      console.error("Failed to select project:", err);
    }
  };

  return (
    <div className="h-full flex text-sm selection:bg-primary/30">
      
      {/* Activity Bar (Far Left) */}
      <div className="w-12 bg-[#333333] flex flex-col items-center py-2 space-y-4 border-r border-border shrink-0">
        <button className={`p-2 rounded ${activeTab === 'explorer' ? 'text-primary' : 'text-text-muted hover:text-text'}`} onClick={() => setActiveTab('explorer')}>
          <Folder size={24} />
        </button>
        <button className={`p-2 rounded ${activeTab === 'kanban' ? 'text-primary' : 'text-text-muted hover:text-text'}`} onClick={() => setActiveTab('kanban')}>
          <ListTodo size={24} />
        </button>
        <div className="flex-1"></div>
        <button className={`p-2 rounded ${activeTab === 'settings' ? 'text-primary' : 'text-text-muted hover:text-text'}`} onClick={() => setActiveTab('settings')}>
          <Settings size={24} />
        </button>
      </div>

      {/* Main IDE Layout */}
      <div className="flex-1 h-full bg-background flex flex-col">
        {/* Header */}
        <header className="h-8 bg-[#3c3c3c] flex items-center justify-center text-xs text-text-muted border-b border-border shadow-sm">
          Sandman - Project: {activeProject || 'Not Selected'}
        </header>

        {/* Resizable Layout */}
        <div className="flex-1 overflow-hidden">
          {activeTab === 'settings' ? (
            <SettingsView />
          ) : (
            <Group orientation="horizontal">
              
              {/* Side Bar */}
              <Panel defaultSize={20} minSize={15} className="bg-surface border-r border-border flex flex-col min-w-0">
                <div className="px-4 py-2 uppercase text-xs font-semibold tracking-wider text-text-muted">
                  {activeTab === 'kanban' ? 'Stories' : 'Explorer'}
                </div>
                <div className="flex-1 flex flex-col min-h-0">
                  {activeProject ? (
                    activeTab === 'explorer' ? (
                      <FileExplorer rootPath={activeProject} />
                    ) : (
                      <div className="p-2">
                        <span className="text-text-muted text-xs font-mono break-all">{activeProject}</span>
                      </div>
                    )
                  ) : (
                    <div className="p-2">
                      <span className="text-text-muted text-xs italic">No folder connected.</span>
                    </div>
                  )}
                </div>

                {activeProject && <SidebarChat />}
              </Panel>
              
              <Separator className="w-1 bg-border hover:bg-primary/50 transition-colors" />

              {/* Editor Group + Bottom Panel Component */}
              <Panel defaultSize={80} className="flex flex-col">
                <Group orientation="vertical">
                  
                  {/* Editor View */}
                  <Panel defaultSize={70} className="bg-background relative">
                    {/* Tabs */}
                    <div className="h-9 flex bg-surface">
                      <div className="px-4 border-t border-t-primary border-r border-border bg-background flex items-center gap-2 cursor-pointer text-text text-sm">
                        <span className="text-blue-400">#</span> {activeTab === 'kanban' ? 'kanban_board' : 'project_explorer'}
                      </div>
                    </div>
                    {/* Content */}
                    <div className="absolute inset-0 top-9">
                      {!activeProject ? (
                        <div className="h-full flex flex-col items-center justify-center text-text-muted text-center p-8">
                          <h2 className="text-2xl font-bold mb-4 text-text tracking-wide">Welcome to Sandman</h2>
                          <p className="mb-6 max-w-md">Select a local repository to unleash autonomous AI agents on your codebase.</p>
                          <button 
                            onClick={handleSelectProject}
                            className="bg-primary hover:bg-primary/90 text-white px-6 py-2 rounded font-medium shadow transition-colors cursor-pointer"
                          >
                            Open Folder...
                          </button>
                        </div>
                      ) : activeTab === 'kanban' ? (
                        <KanbanBoard />
                      ) : (
                        <div className="h-full flex items-center justify-center text-text-muted text-center p-8">
                          <div>
                            <h2 className="text-xl mb-2 text-text">Explorer Mode</h2>
                            <p>Select a file from the sidebar to view its code.</p>
                          </div>
                        </div>
                      )}
                    </div>
                  </Panel>

                  <Separator className="h-1 bg-border hover:bg-primary/50 transition-colors" />
                  
                  {/* Bottom Terminal Panel */}
                  <Panel defaultSize={30} minSize={10} className="bg-[#1e1e1e] flex flex-col border-t border-border">
                    <ul className="h-8 flex px-4 items-center gap-4 text-xs font-medium tracking-wide uppercase text-text-muted border-b border-border">
                      <li className="cursor-pointer border-b border-transparent hover:text-text">Output</li>
                      <li className="cursor-pointer border-b border-primary text-text">Terminal</li>
                    </ul>
                    <div className="flex-1 p-2 overflow-hidden flex items-stretch">
                      <TerminalPanel />
                    </div>
                  </Panel>

                </Group>
              </Panel>
            </Group>
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
