import React, { useState, useEffect } from 'react';
import { Bot, Play, CheckCircle, CircleDashed, AlertOctagon, TerminalSquare, Plus, CornerDownLeft, Trash2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

type StoryStatus = 'Raw Requirements' | 'Clarification Required' | 'Backlog' | 'To Do' | 'In Progress' | 'Review' | 'Done';

interface Story {
  id: string;
  title: string;
  description?: string;
  status: StoryStatus;
  ai_ready: number;
  ai_hold: number;
  agent?: 'Story' | 'Builder' | 'Reviewer';
  state?: 'idle' | 'processing' | 'failed' | 'success';
}

const COLUMNS: StoryStatus[] = ['Raw Requirements', 'Clarification Required', 'Backlog', 'To Do', 'In Progress', 'Review', 'Done'];

export function KanbanBoard() {
  const [stories, setStories] = useState<Story[]>([]);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [activeModalStory, setActiveModalStory] = useState<Story | null>(null);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [activeAIColumns, setActiveAIColumns] = useState<Set<StoryStatus>>(new Set(['Raw Requirements', 'Clarification Required', 'To Do', 'In Progress', 'Review']));

  const questions = activeModalStory?.description 
    ? activeModalStory.description
        .split(/Clarifying Questions:?/i)[1]
        ?.split('\n')
        .filter(l => l.trim().match(/^[-*•?]|^\d+\./))
        .map(l => l.trim())
    : [];

  const [config, setConfig] = useState<any>(null);
  const [storyTasks, setStoryTasks] = useState<Record<string, any[]>>({});

  const loadConfig = async () => {
    try {
      const c = await invoke('get_config');
      setConfig(c);
    } catch (err) {
      console.error(err);
    }
  };

  const fetchStoryTasks = async (ids: string[]) => {
    const results: Record<string, any[]> = {};
    await Promise.all(
      ids.map(async (id) => {
        try {
          const tasks = await invoke<any[]>('get_story_tasks', { storyId: id });
          results[id] = tasks;
        } catch { results[id] = []; }
      })
    );
    setStoryTasks(prev => ({ ...prev, ...results }));
  };

  const handleSetColumnStrategy = async (status: StoryStatus, providerId: string) => {
    try {
      await invoke('set_column_strategy', { status, providerId });
      loadConfig();
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadConfig();
    fetchStories();
    const storyInterval = setInterval(fetchStories, 3000);
    return () => clearInterval(storyInterval);
  }, []);

  // Poll tasks for all stories in active SDLC stages
  useEffect(() => {
    const activeIds = stories
      .filter(s => ['To Do', 'In Progress', 'Review'].includes(s.status))
      .map(s => s.id);
    if (activeIds.length > 0) fetchStoryTasks(activeIds);
  }, [stories]);

  const fetchStories = async () => {
    try {
      const data = await invoke<Story[]>('get_stories');
      setStories(data);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAnswerQuestions = async () => {
    if (!activeModalStory) return;
    try {
      setStories((prev) => prev.map(s => s.id === activeModalStory.id ? { ...s, state: 'processing' } : s));
      
      const combinedAnswer = questions && questions.length > 0
        ? questions.map(q => `Question: ${q}\nAnswer: ${answers[q] || "N/A"}`).join('\n\n')
        : Object.values(answers).join('\n');

      if (!combinedAnswer.trim()) return;

      const clarification = `User Answer to Clarifying Questions:\n${combinedAnswer}`;
      await invoke('dispatch_agent', { id: activeModalStory.id, additionalContext: clarification });
      setActiveModalStory(null);
      setAnswers({});
    } catch (e) {
      console.error("Failed to answer questions:", e);
      fetchStories();
    }
  };

  const toggleAIForColumn = (col: StoryStatus) => {
    setActiveAIColumns(prev => {
      const next = new Set(prev);
      if (next.has(col)) next.delete(col);
      else next.add(col);
      return next;
    });
  };

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    try {
      const newStory = await invoke<Story>('create_story', { title: newTitle });
      setStories((prev) => [...prev, newStory]);
      setNewTitle('');
      setIsAdding(false);
      
      // Auto-trigger if column is active and not on hold
      if (activeAIColumns.has(newStory.status) && newStory.ai_hold === 0) {
        handleAIPush(newStory.id);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleAIPush = async (id: string) => {
    try {
      setStories((prev) => prev.map(s => s.id === id ? { ...s, state: 'processing', agent: 'Builder' } : s));
      await invoke('dispatch_agent', { id, additionalContext: null });
    } catch (e) {
      console.error("Failed to dispatch agent:", e);
      fetchStories();
    }
  };

  const handleToggleHold = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const newHold = await invoke<number>('toggle_story_hold', { id });
      setStories((prev) => prev.map(s => s.id === id ? { ...s, ai_hold: newHold } : s));
    } catch (e) {
      console.error("Failed to toggle hold:", e);
    }
  };

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (window.confirm("Are you sure you want to delete this story?")) {
      try {
        await invoke('delete_story', { id });
        setStories((prev) => prev.filter(s => s.id !== id));
      } catch (e) {
        console.error("Failed to delete story:", e);
      }
    }
  };

  useEffect(() => {
    const interval = setInterval(fetchStories, 3000); // Simple polling for agent updates
    return () => clearInterval(interval);
  }, []);

  // Automatic "One by One" Processing Loop
  useEffect(() => {
    const processNext = async () => {
      // Check if any story is currently processing
      const isAnythingProcessing = stories.some(s => s.state === 'processing');
      if (isAnythingProcessing) return;

      // Find the next eligible story
      // Priority: Left-to-right columns, then top-to-bottom cards
      for (const col of COLUMNS) {
        if (!activeAIColumns.has(col)) continue;
        
        const nextInCol = stories.find(s => 
          s.status === col && 
          s.ai_hold === 0 && 
          (!s.state || s.state === 'idle' || s.state === 'failed')
        );

        if (nextInCol) {
          handleAIPush(nextInCol.id);
          break; // Process one by one
        }
      }
    };

    const timer = setTimeout(processNext, 2000);
    return () => clearTimeout(timer);
  }, [stories, activeAIColumns]);

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
      
      // Auto-trigger AI if column is active
      if (activeAIColumns.has(targetStatus)) {
        handleAIPush(id);
      }
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
            <div className="p-3 border-b border-border flex flex-col gap-2">
              <div className="flex items-center justify-between w-full">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] uppercase font-bold text-text-muted tracking-wide truncate max-w-[120px]">{col}</span>
                  <span className="text-[10px] text-text-muted bg-background px-1.5 py-0.5 rounded-full font-mono">{colStories.length}</span>
                </div>
                
                <div className="flex items-center gap-1.5">
                  <button 
                    onClick={() => toggleAIForColumn(col)}
                    className={`p-1.5 rounded-md transition-all ${
                      activeAIColumns.has(col) 
                        ? 'text-green-500 bg-green-500/5 hover:bg-green-500/10' 
                        : 'text-text-muted hover:bg-white/5'
                    }`}
                    title={activeAIColumns.has(col) ? "AI Processing Active" : "AI Processing Paused"}
                  >
                    {activeAIColumns.has(col) ? <Play size={12} fill="currentColor" className="animate-pulse-slow" /> : <div className="w-3 h-3 border-2 border-text-muted rounded-[2px]" />}
                  </button>
                </div>
              </div>

              {/* Model Strategy Selector */}
              {col !== 'Done' && col !== 'Backlog' && (
                <div className="flex items-center gap-2">
                  <span className="text-[9px] text-text-muted uppercase font-semibold">Brain:</span>
                  <select 
                    value={config?.column_strategies?.[col] || (config?.providers?.find((p: any) => p.active)?.id || '')}
                    onChange={(e) => handleSetColumnStrategy(col, e.target.value)}
                    className="flex-1 bg-background border border-border text-[10px] py-0.5 px-1 rounded-sm text-text-muted focus:outline-none focus:border-primary/50 cursor-pointer appearance-none hover:text-text transition-colors"
                  >
                    {config?.providers?.map((p: any) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                </div>
              )}
            </div>

            {/* Column Body */}
            <div className="flex-1 p-2 space-y-2 overflow-y-auto">
              
              {/* Raw Requirements Add Button */}
              {col === 'Raw Requirements' && !isAdding && (
                <button 
                  onClick={() => setIsAdding(true)}
                  className="w-full py-2 border border-dashed border-border rounded text-text-muted hover:text-text hover:border-primary/50 text-xs flex items-center justify-center gap-2 transition-colors"
                >
                  <Plus size={14} /> New Story
                </button>
              )}
              {col === 'Raw Requirements' && isAdding && (
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
                  onClick={() => setActiveModalStory(story)}
                  className={`bg-background border rounded p-3 cursor-pointer active:cursor-grabbing shadow-sm hover:border-primary/50 transition-colors
                    ${draggingId === story.id ? 'opacity-50 border-dashed border-primary' : 'border-border'}
                    ${story.state === 'failed' ? 'border-red-500/50' : ''}
                    ${story.state === 'success' ? 'border-green-500/30' : ''}
                  `}
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs font-mono text-blue-400">{story.id}</span>
                    <div className="flex gap-1">
                      {/* Hold / Resume Toggle */}
                      <button 
                        onClick={(e) => handleToggleHold(story.id, e)}
                        className={`text-[10px] flex items-center gap-1 px-1.5 py-0.5 rounded border transition-colors ${
                          story.ai_hold === 1 
                            ? 'bg-orange-900/40 text-orange-400 border-orange-700/50 hover:bg-orange-800/60'
                            : 'bg-green-900/40 text-green-400 border-green-700/50 hover:bg-green-800/60'
                        }`}
                        title={story.ai_hold === 1 ? "Resume AI Processing" : "Put on Hold"}
                      >
                         {story.ai_hold === 1 ? <Play size={10} fill="currentColor" /> : <div className="w-2.5 h-2.5 bg-green-400 rounded-sm" />}
                         {story.ai_hold === 1 ? 'Paused' : 'Ready'}
                      </button>

                      {story.state === 'processing' && (
                        <span 
                          className="text-[10px] flex items-center gap-1 px-1.5 py-0.5 rounded border bg-yellow-900/40 text-yellow-400 border-yellow-700/50 animate-pulse"
                        >
                          <TerminalSquare size={10} /> Running...
                        </span>
                      )}
                      
                      <button 
                         onClick={(e) => handleDelete(story.id, e)}
                         className="text-[10px] flex items-center gap-1 px-1.5 py-0.5 rounded border border-border hover:bg-red-900/40 hover:text-red-400 hover:border-red-700/50 text-text-muted transition-colors transition-opacity pointer-events-auto"
                         title="Delete Story"
                      >
                         <Trash2 size={10} />
                      </button>
                    </div>
                  </div>

                  <p className="text-sm text-text leading-snug mb-2 pointer-events-none">{story.title}</p>
                  {story.description && (
                    <div className="text-[10px] text-text-muted line-clamp-3 mb-2 bg-surface/50 p-1.5 rounded border border-border/50">
                      {story.description.replace(/# Title:.*?\n/, '').trim()}
                    </div>
                  )}

                  {/* AI Task Progress */}
                  {storyTasks[story.id] && storyTasks[story.id].length > 0 && (() => {
                    const tasks = storyTasks[story.id];
                    const done = tasks.filter(t => t.completed).length;
                    const total = tasks.length;
                    const pct = Math.round((done / total) * 100);
                    return (
                      <div className="mb-2" onClick={e => e.stopPropagation()}>
                        <div className="flex items-center justify-between text-[9px] text-text-muted mb-1">
                          <span className="font-semibold uppercase tracking-wide">Progress</span>
                          <span className="font-mono">{done}/{total}</span>
                        </div>
                        <div className="h-1 bg-surface rounded-full overflow-hidden">
                          <div
                            className="h-full bg-gradient-to-r from-blue-500 to-primary transition-all duration-500"
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                        {tasks.length <= 5 && (
                          <ul className="mt-1.5 space-y-0.5">
                            {tasks.map((t: any) => (
                              <li key={t.id} className="flex items-start gap-1.5 text-[9px] text-text-muted">
                                <span className={`mt-px flex-shrink-0 w-2.5 h-2.5 rounded-sm border flex items-center justify-center ${
                                  t.completed ? 'bg-green-500/20 border-green-500/50' : 'border-border'
                                }`}>
                                  {t.completed && <CheckCircle size={7} className="text-green-400" />}
                                </span>
                                <span className={t.completed ? 'line-through opacity-50' : ''}>{t.title}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    );
                  })()}

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

      {/* Story Detail / Clarification Modal */}
      {activeModalStory && (
        <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="bg-surface border border-border rounded-lg max-w-2xl w-full max-h-[90vh] flex flex-col shadow-2xl overflow-hidden animate-in fade-in zoom-in duration-200">
            <div className="p-4 border-b border-border flex items-center justify-between">
              <h3 className="font-semibold text-text flex items-center gap-2">
                <Bot size={18} className="text-primary" /> Story Details: {activeModalStory.id}
              </h3>
              <button onClick={() => { setActiveModalStory(null); setAnswers({}); }} className="text-text-muted hover:text-text transition-colors">
                <Plus size={18} className="rotate-45" />
              </button>
            </div>
            
            <div className="flex-1 overflow-y-auto p-4 space-y-6">
              {/* Title Section */}
              <div className="space-y-1">
                <label className="text-[10px] uppercase font-bold text-text-muted tracking-widest">Title</label>
                <div className="text-sm font-medium p-3 bg-background rounded-md border border-border">{activeModalStory.title}</div>
              </div>
              
              {/* Status Section */}
              <div className="flex gap-4">
                 <div className="flex-1 space-y-1">
                   <label className="text-[10px] uppercase font-bold text-text-muted tracking-widest">Status</label>
                   <div className="text-xs px-2 py-1 bg-[#3c3c3c] text-text rounded-md w-fit border border-border">{activeModalStory.status}</div>
                 </div>
                 {activeModalStory.agent && (
                   <div className="flex-1 space-y-1 text-right">
                     <label className="text-[10px] uppercase font-bold text-text-muted tracking-widest">Agent Assigned</label>
                     <div className="text-xs text-primary font-semibold">{activeModalStory.agent}</div>
                   </div>
                 )}
              </div>

              {/* Description / Content Section */}
              <div className="space-y-2">
                <label className="text-[10px] uppercase font-bold text-text-muted tracking-widest">
                  {activeModalStory.status === 'Clarification Required' ? 'AI Review & Context' : 'Story Description'}
                </label>
                <div className="text-sm p-3 bg-background rounded-md border border-border whitespace-pre-wrap font-mono prose-invert text-text leading-relaxed">
                  {activeModalStory.status === 'Clarification Required' 
                    ? activeModalStory.description?.split(/Clarifying Questions:?/i)[0] || activeModalStory.description
                    : activeModalStory.description || "No description provided yet."}
                </div>
              </div>
              
              {/* Question & Answer Forms for Clarification Required stories */}
              {activeModalStory.status === 'Clarification Required' && (
                <div className="space-y-4 pt-4 border-t border-border">
                  <label className="text-[10px] uppercase font-bold text-primary tracking-widest block">Clarifying Questions & Your Answers</label>
                  
                  {questions && questions.length > 0 ? (
                    questions.map((q, i) => (
                      <div key={i} className="space-y-2 group">
                        <p className="text-xs text-text group-hover:text-primary transition-colors font-medium ml-1">Q: {q}</p>
                        <textarea 
                            placeholder="Your answer..."
                            value={answers[q] || ''}
                            onChange={(e) => setAnswers(prev => ({ ...prev, [q]: e.target.value }))}
                            className="w-full bg-background border border-border rounded-md p-3 text-sm text-text outline-none focus:border-primary/50 transition-all h-20"
                        />
                      </div>
                    ))
                  ) : (
                    <textarea 
                      autoFocus
                      placeholder="Provide more details or answer any questions AI might have..."
                      value={answers['default'] || ''}
                      onChange={(e) => setAnswers(prev => ({ ...prev, 'default': e.target.value }))}
                      className="w-full h-32 bg-background border border-border rounded-md p-3 text-sm text-text outline-none focus:border-primary/50 transition-colors"
                    />
                  )}
                </div>
              )}
            </div>
            
            <div className="p-4 border-t border-border flex justify-end gap-3 bg-surface/50">
              <button 
                onClick={() => { setActiveModalStory(null); setAnswers({}); }}
                className="px-4 py-1.5 rounded text-sm text-text hover:bg-white/5 transition-colors"
              >
                Close
              </button>
              {activeModalStory.status === 'Clarification Required' && (
                <button 
                  onClick={handleAnswerQuestions}
                  disabled={Object.values(answers).every(v => !v.trim())}
                  className="px-4 py-1.5 bg-primary hover:bg-primary/90 disabled:opacity-50 text-white rounded text-sm font-semibold flex items-center gap-2 shadow-lg shadow-primary/20 transition-all hover:scale-105 active:scale-95"
                >
                  <Play size={14} /> Submit Answers to AI
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
