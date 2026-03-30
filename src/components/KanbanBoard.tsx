import React, { useState, useEffect } from 'react';
import { Bot, Play, CheckCircle, CircleDashed, AlertOctagon, TerminalSquare, Plus, CornerDownLeft } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

type StoryStatus = 'Backlog' | 'To Do' | 'In Progress' | 'Review' | 'Done';

interface Story {
  id: string;
  title: string;
  status: StoryStatus;
  ai_ready: number;
  agent?: 'Story' | 'Builder' | 'Reviewer';
  state?: 'idle' | 'processing' | 'failed' | 'success';
}

const COLUMNS: StoryStatus[] = ['Backlog', 'To Do', 'In Progress', 'Review', 'Done'];

export function KanbanBoard() {
  const [stories, setStories] = useState<Story[]>([]);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');

  useEffect(() => {
    fetchStories();
  }, []);

  const fetchStories = async () => {
    try {
      const data = await invoke<Story[]>('get_stories');
      setStories(data);
    } catch (e) {
      console.error(e);
    }
  };

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    try {
      const newStory = await invoke<Story>('create_story', { title: newTitle });
      setStories((prev) => [...prev, newStory]);
      setNewTitle('');
      setIsAdding(false);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAIPush = async (id: string) => {
    try {
      setStories((prev) => prev.map(s => s.id === id ? { ...s, state: 'processing', agent: 'Builder' } : s));
      await invoke('dispatch_agent', { id });
    } catch (e) {
      console.error("Failed to dispatch agent:", e);
      fetchStories();
    }
  };

  useEffect(() => {
    const interval = setInterval(fetchStories, 3000); // Simple polling for agent updates
    return () => clearInterval(interval);
  }, []);

  const handleDragStart = (e: React.DragEvent, id: string) => {
    setDraggingId(id);
    e.dataTransfer.setData('text/plain', id);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = async (e: React.DragEvent, targetStatus: StoryStatus) => {
    e.preventDefault();
    const id = e.dataTransfer.getData('text/plain');
    if (!id || id === draggingId) return;

    // Optimistically update UI
    setStories((prev) => prev.map((s) => (s.id === id ? { ...s, status: targetStatus } : s)));
    setDraggingId(null);

    // Persist to sqlite DB
    try {
      await invoke('update_story_status', { id, status: targetStatus });
    } catch (err) {
      console.error("Failed to update status in DB", err);
      fetchStories(); // revert on failure
    }
  };

  return (
    <div className="flex h-full w-full bg-background overflow-x-auto p-4 gap-4">
      {COLUMNS.map((col) => {
        const colStories = stories.filter((s) => s.status === col);

        return (
          <div
            key={col}
            className="flex-shrink-0 flex flex-col bg-surface border border-border rounded-md w-72 h-full"
            onDragOver={handleDragOver}
            onDrop={(e) => handleDrop(e, col)}
          >
            {/* Column Header */}
            <div className="p-3 border-b border-border flex items-center justify-between pointer-events-none">
              <span className="text-xs uppercase font-semibold text-text-muted tracking-wider">{col}</span>
              <span className="text-xs text-text-muted bg-background px-2 py-0.5 rounded-full">{colStories.length}</span>
            </div>

            {/* Column Body */}
            <div className="flex-1 p-2 space-y-2 overflow-y-auto">
              
              {/* Backlog Add Button */}
              {col === 'Backlog' && !isAdding && (
                <button 
                  onClick={() => setIsAdding(true)}
                  className="w-full py-2 border border-dashed border-border rounded text-text-muted hover:text-text hover:border-primary/50 text-xs flex items-center justify-center gap-2 transition-colors"
                >
                  <Plus size={14} /> New Story
                </button>
              )}
              {col === 'Backlog' && isAdding && (
                <div className="bg-background border border-primary/50 rounded p-2 flex flex-col gap-2">
                  <input
                    type="text"
                    autoFocus
                    value={newTitle}
                    onChange={(e) => setNewTitle(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleCreate();
                      if (e.key === 'Escape') setIsAdding(false);
                    }}
                    placeholder="Describe a task..."
                    className="bg-surface text-sm text-text outline-none p-1.5 rounded"
                  />
                  <div className="flex gap-2">
                    <button onClick={handleCreate} className="bg-primary hover:bg-primary/90 text-white rounded px-2 py-1 text-xs flex-1 flex items-center justify-center gap-1">
                      <CornerDownLeft size={12} /> Save
                    </button>
                    <button onClick={() => setIsAdding(false)} className="bg-surface hover:bg-[#3c3c3c] text-text-muted rounded px-2 py-1 text-xs">
                      Cancel
                    </button>
                  </div>
                </div>
              )}

              {colStories.map((story) => (
                <div
                  key={story.id}
                  draggable
                  onDragStart={(e) => handleDragStart(e, story.id)}
                  className={`bg-background border rounded p-3 cursor-grab active:cursor-grabbing shadow-sm hover:border-primary/50 transition-colors
                    ${draggingId === story.id ? 'opacity-50 border-dashed border-primary' : 'border-border'}
                    ${story.state === 'failed' ? 'border-red-500/50' : ''}
                    ${story.state === 'success' ? 'border-green-500/30' : ''}
                  `}
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs font-mono text-blue-400">{story.id}</span>
                    <div className="flex gap-1">
                      {story.ai_ready === 1 && (
                        <span 
                          onClick={() => handleAIPush(story.id)}
                          className={`text-[10px] flex items-center gap-1 px-1.5 py-0.5 rounded border cursor-pointer pointer-events-auto transition-colors ${
                            story.state === 'processing' 
                              ? 'bg-yellow-900/40 text-yellow-400 border-yellow-700/50 animate-pulse'
                              : 'bg-green-900/40 text-green-400 border-green-700/50 hover:bg-green-800/60'
                          }`}
                        >
                          <Play size={10} /> {story.state === 'processing' ? 'Agent Running...' : 'AI Ready'}
                        </span>
                      )}
                    </div>
                  </div>

                  <p className="text-sm text-text leading-snug mb-3 pointer-events-none">{story.title}</p>

                  <div className="flex items-center justify-between text-xs text-text-muted">
                    {/* Agent Status Icon */}
                    <div className="flex items-center gap-1">
                      {story.state === 'processing' && <TerminalSquare size={13} className="text-yellow-500 animate-pulse" />}
                      {story.state === 'failed' && <AlertOctagon size={13} className="text-red-500" />}
                      {story.state === 'success' && <CheckCircle size={13} className="text-green-500" />}
                      {!story.state && <CircleDashed size={13} />}
                      
                      <span className="font-medium">
                        {story.agent ? story.agent : 'Unassigned'}
                      </span>
                    </div>

                    {/* AI Avatar */}
                    {story.agent && (
                      <div className="h-5 w-5 rounded bg-[#3c3c3c] flex items-center justify-center text-text border border-border">
                        <Bot size={12} />
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}
