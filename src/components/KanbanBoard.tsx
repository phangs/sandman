import React, { useState, useEffect } from 'react';
import { Bot, Play, CheckCircle, CircleDashed, AlertOctagon, TerminalSquare, Plus, Trash2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

type StoryStatus = 'Raw Requirements' | 'Clarification Required' | 'Backlog' | 'To Do' | 'In Progress' | 'Review' | 'Testing' | 'Done';

interface Story {
  id: string;
  title: string;
  description?: string;
  reviewer_feedback?: string;
  status: StoryStatus;
  ai_ready: number;
  ai_hold: number;
  skip_clarification: number;
  agent?: 'Story' | 'Builder' | 'Reviewer' | 'Tester';
  state?: 'idle' | 'processing' | 'failed' | 'success';
}

const COLUMNS: StoryStatus[] = ['Raw Requirements', 'Clarification Required', 'Backlog', 'To Do', 'In Progress', 'Review', 'Testing', 'Done'];

export function KanbanBoard() {
  const [stories, setStories] = useState<Story[]>([]);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [skipClarification, setSkipClarification] = useState(false);
  const [activeModalStory, setActiveModalStory] = useState<Story | null>(null);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [activeAIColumns, setActiveAIColumns] = useState<Set<StoryStatus>>(new Set(['Raw Requirements', 'Clarification Required', 'Backlog', 'To Do', 'In Progress', 'Review', 'Testing']));

  const modalStory = activeModalStory ? stories.find(s => s.id === activeModalStory.id) || activeModalStory : null;

  const questions = modalStory?.description 
    ? modalStory.description
        .split(/Clarifying Questions:?/i)[1]
        ?.split('\n')
        .filter(l => l.trim().match(/^[-*•?]|^\d+\./))
        .map(l => l.trim())
    : [];

  useEffect(() => {
    fetchStories();
    const storyInterval = setInterval(fetchStories, 3000);
    return () => clearInterval(storyInterval);
  }, []);

  const fetchStories = async () => {
    try {
      const data = await invoke<Story[]>('get_stories');
      setStories(data);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAnswerQuestions = async () => {
    if (!modalStory) return;
    try {
      setStories((prev) => prev.map(s => s.id === modalStory.id ? { ...s, state: 'processing' } : s));
      
      const combinedAnswer = questions && questions.length > 0
        ? questions.map(q => `Question: ${q}\nAnswer: ${answers[q] || "N/A"}`).join('\n\n')
        : Object.values(answers).join('\n');

      if (!combinedAnswer.trim()) return;

      const clarification = `User Answer to Clarifying Questions:\n${combinedAnswer}`;
      await invoke('dispatch_agent', { id: modalStory.id, additionalContext: clarification });
      setActiveModalStory(null);
      setAnswers({});
    } catch (e) {
      console.error("Failed to answer questions:", e);
      fetchStories();
    }
  };

  const toggleAIForColumn = async (col: StoryStatus) => {
    const isActivating = !activeAIColumns.has(col);
    setActiveAIColumns(prev => {
      const next = new Set(prev);
      if (next.has(col)) next.delete(col);
      else next.add(col);
      return next;
    });

    if (isActivating) {
      try {
        await invoke('clear_column_state', { status: col });
        fetchStories();
      } catch (e) {
        console.error("Failed to clear column state:", e);
      }
    }
  };

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    try {
      const newStory = await invoke<Story>('create_story', { 
        title: newTitle,
        skip_clarification: skipClarification ? 1 : 0
      });
      setStories((prev) => [...prev, newStory]);
      setNewTitle('');
      setSkipClarification(false);
      setIsAdding(false);
      if (activeAIColumns.has(newStory.status) && newStory.ai_hold === 0) {
        handleAIPush(newStory.id);
      }
    } catch (e) {
      console.error(e);
    }
  };

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
    if (!id) return;

    setStories((prev) => prev.map((s) => (s.id === id ? { ...s, status: targetStatus } : s)));
    setDraggingId(null);

    try {
      if (['To Do', 'In Progress', 'Review'].includes(targetStatus)) {
        await invoke('update_story_status', { id, status: targetStatus });
        await invoke('update_story_ready', { id, ready: true }); 
      } else {
        await invoke('update_story_status', { id, status: targetStatus });
      }
      fetchStories();
      if (activeAIColumns.has(targetStatus)) {
        handleAIPush(id);
      }
    } catch (err) {
      console.error("Failed to update status in DB", err);
      fetchStories();
    }
  };

  const handleAIPush = async (id: string) => {
    try {
      setStories((prev) => prev.map(s => s.id === id ? { ...s, state: 'processing' } : s));
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
    const tick = async () => {
      let fresh: Story[] = [];
      try {
        fresh = await invoke<Story[]>('get_stories');
        setStories(fresh);
      } catch (e) {
        console.error(e);
        return;
      }
      const isProcessing = fresh.some(s => s.state === 'processing');
      if (isProcessing) return;
      for (const col of COLUMNS) {
        if (!activeAIColumns.has(col)) continue;
        const next = fresh.find(s =>
          s.status === col &&
          s.ai_hold === 0 &&
          (col === 'Raw Requirements' || s.ai_ready === 1) &&
          s.state === 'idle'
        );
        if (next) {
          handleAIPush(next.id);
          break;
        }
      }
    };
    const interval = setInterval(tick, 4000);
    return () => clearInterval(interval);
  }, [activeAIColumns]);

  return (
    <div className="flex flex-col h-full bg-[#121212] text-text font-sans selection:bg-primary/30 antialiased overflow-hidden relative">
      <div className="flex-shrink-0 flex items-center justify-between p-4 border-b border-border/60 bg-surface/30 backdrop-blur-sm z-10">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary/10 rounded-lg text-primary ring-1 ring-primary/20">
              <Bot size={20} />
            </div>
            <div>
              <h1 className="text-sm font-bold tracking-tight text-white/90">Autonomous SDLC</h1>
              <p className="text-[10px] text-text-muted font-medium opacity-50 uppercase tracking-widest">{activeAIColumns.size > 0 ? 'Agent Loop: Active' : 'Agent Loop: Paused'}</p>
            </div>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-x-auto p-4 custom-scrollbar">
        <div className="flex gap-4 h-full min-w-max">
          {COLUMNS.map((col) => {
            const colStories = stories.filter((s) => s.status === col);
            return (
              <div
                key={col}
                onDragOver={handleDragOver}
                onDrop={(e) => handleDrop(e, col)}
                className="w-72 flex flex-col h-full rounded-xl bg-surface/20 border border-border/40 backdrop-blur-sm shadow-inner group/column transition-all duration-300 hover:bg-surface/30 px-2.5 pb-2.5 pt-0"
              >
                <div className="flex items-center justify-between p-3.5 mb-2 sticky top-0 bg-transparent z-10">
                  <div className="flex items-center gap-2">
                    <h2 className="text-xs font-bold uppercase tracking-widest text-[#999] group-hover/column:text-white/100 transition-colors pointer-events-none">{col}</h2>
                    <span className="text-[10px] bg-white/5 border border-white/10 px-2 py-0.5 rounded-full font-mono text-text-muted">{colStories.length}</span>
                  </div>
                  <div className="flex items-center gap-1">
                     <button 
                        onClick={() => toggleAIForColumn(col)}
                        className={`p-1.5 rounded-md transition-all hover:scale-110 active:scale-95 ${activeAIColumns.has(col) ? 'text-primary bg-primary/10 hover:bg-primary/20 shadow-[0_0_10px_rgba(59,130,246,0.2)]' : 'text-text-muted hover:text-white bg-white/5'}`}
                        title={activeAIColumns.has(col) ? `Pause AI in ${col}` : `Force Start / Reset AI in ${col}`}
                     >
                       {activeAIColumns.has(col) ? <AlertOctagon size={14} /> : <Play size={14} fill="currentColor" />}
                     </button>
                    {col === 'Raw Requirements' && (
                      <button onClick={() => setIsAdding(true)} className="p-1.5 hover:bg-white/5 text-text-muted hover:text-white rounded-md transition-colors">
                        <Plus size={14} />
                      </button>
                    )}
                  </div>
                </div>

                <div className="flex-1 overflow-y-auto custom-scrollbar flex flex-col gap-3 min-h-[100px]">
                  {colStories.length === 0 && (
                    <div className="flex-1 flex flex-col items-center justify-center gap-2 opacity-10 pointer-events-none border-2 border-dashed border-white/50 rounded-lg m-1">
                      <div className="w-8 h-8 rounded-full border-2 border-current flex items-center justify-center">
                        <CircleDashed size={16} />
                      </div>
                      <span className="text-[10px] uppercase font-bold tracking-wider">Empty Column</span>
                    </div>
                  )}

                  {colStories.map((story) => (
                    <div
                      key={story.id}
                      draggable
                      onDragStart={(e) => handleDragStart(e, story.id)}
                      onClick={() => setActiveModalStory(story)}
                      className={`bg-[#1e1e1e]/80 border border-border/40 rounded-xl p-3 cursor-pointer active:cursor-grabbing shadow-lg hover:border-primary/50 hover:shadow-primary/5 hover:translate-y-[-1px] active:translate-y-[1px] transition-all duration-200 group/card relative
                        ${draggingId === story.id ? 'opacity-50 ring-2 ring-primary border-transparent' : 'border-border/40'}
                        ${story.state === 'failed' ? 'border-red-500/40 bg-red-900/5 shadow-red-500/5' : ''}
                        ${story.state === 'success' ? 'border-green-500/10' : ''}
                      `}
                    >
                      <div className="flex items-center justify-between mb-2.5">
                        <div className="flex items-center gap-2">
                           <span className="text-[9px] font-bold py-0.5 px-1.5 bg-black/40 text-blue-400 rounded-md border border-blue-400/20 tracking-wider font-mono uppercase">{story.id}</span>
                           {story.agent && (
                             <span className="text-[9px] text-primary/80 font-bold uppercase tracking-widest">{story.agent}</span>
                           )}
                        </div>
                        <div className="flex gap-1.5 items-center" onClick={e => e.stopPropagation()}>
                           <button 
                             onClick={(e) => handleToggleHold(story.id, e)}
                             className={`text-[9px] font-bold flex items-center gap-1.5 px-2 py-0.5 rounded-full border border-border transition-all ${story.ai_hold === 1 ? 'bg-background hover:bg-white/5 text-text-muted' : 'bg-green-500/10 border-green-500/20 text-green-400'}`}
                             title={story.ai_hold === 1 ? "Resume AI Processing" : "Put on Hold"}
                           >
                              {story.ai_hold === 1 ? <Play size={8} fill="currentColor" /> : <div className="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse shadow-[0_0_8px_rgba(74,222,128,0.5)]" />}
                              {story.ai_hold === 1 ? 'Paused' : 'Ready'}
                           </button>

                           {story.state === 'processing' && (
                             <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-yellow-400/10 border border-yellow-400/20">
                               <div className="w-1.5 h-1.5 bg-yellow-400 rounded-full animate-ping" />
                             </div>
                           )}
                           
                           <button 
                              onClick={(e) => handleDelete(story.id, e)}
                              className="w-5 h-5 flex items-center justify-center rounded-full border border-border text-text-muted opacity-0 group-hover/card:opacity-100 hover:bg-red-500/20 hover:text-red-400 hover:border-red-500/30 transition-all pointer-events-auto"
                              title="Delete Story"
                           >
                              <Trash2 size={10} />
                           </button>
                        </div>
                      </div>

                      <p className="text-xs font-bold text-white/90 leading-relaxed mb-3 pointer-events-none tracking-tight">{story.title}</p>
                      
                      {story.description && (
                        <div className="text-[10px] text-text-muted line-clamp-2 mb-2 bg-black/20 p-2 rounded-lg border border-white/5 transition-all group-hover/card:bg-black/30 group-hover/card:border-white/10">
                          <div className="uppercase text-[8px] font-bold opacity-30 tracking-widest mb-1">Mission</div>
                          {story.description.replace(/# Title:.*?\n/, '').trim()}
                        </div>
                      )}

                      {story.reviewer_feedback && (
                        <div className="text-[10px] text-text-muted line-clamp-2 mb-3 bg-blue-500/5 p-2 rounded-lg border border-blue-500/20">
                           <div className="uppercase text-[8px] font-bold text-blue-400/60 tracking-widest mb-1 flex items-center gap-1">
                             <TerminalSquare size={8} /> Status
                           </div>
                           {story.reviewer_feedback.trim()}
                        </div>
                      )}

                      <div className="flex items-center justify-between text-[9px] text-[#777] mb-1.5 px-0.5">
                         <div className="flex items-center gap-1">
                            {story.state === 'failed' && <AlertOctagon size={10} className="text-red-500" />}
                            {story.state === 'success' && <CheckCircle size={10} className="text-green-500" />}
                            {story.state === 'processing' && <TerminalSquare size={10} className="text-yellow-500 animate-pulse" />}
                            <span className="font-bold opacity-50 uppercase tracking-widest">{story.state || 'Idle'}</span>
                         </div>
                      </div>
                    </div>
                  ))}

                  {col === 'Raw Requirements' && isAdding && (
                    <div className="bg-surface/50 border border-primary/40 rounded-xl p-3 flex flex-col gap-3 shadow-2xl animate-in fade-in zoom-in duration-200">
                      <textarea
                        autoFocus
                        rows={6}
                        value={newTitle}
                        onChange={(e) => setNewTitle(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) handleCreate();
                          if (e.key === 'Escape') setIsAdding(false);
                        }}
                        placeholder="Paste raw requirements, bug reports, or feature requests here... (Ctrl+Enter to create)"
                        className="bg-black/40 text-xs text-text border border-white/5 outline-none p-3 rounded-lg focus:border-primary/50 transition-all font-medium resize-none leading-relaxed"
                      />
                      
                      {/* Skip Clarification Toggle */}
                      <label className="flex items-center gap-3 px-1 cursor-pointer group select-none">
                        <div 
                          onClick={() => setSkipClarification(!skipClarification)}
                          className={`w-8 h-4 rounded-full transition-all duration-300 relative ${skipClarification ? 'bg-primary' : 'bg-white/10'}`}
                        >
                          <div className={`absolute top-0.5 w-3 h-3 bg-white rounded-full transition-all duration-300 ${skipClarification ? 'left-4.5' : 'left-0.5'}`} />
                        </div>
                        <span className="text-[10px] text-text-muted group-hover:text-text transition-colors">Skip Clarifying Questions</span>
                      </label>

                      <div className="flex gap-2 items-center">
                        <button onClick={handleCreate} className="bg-primary hover:bg-primary/90 text-white font-bold rounded-lg px-4 py-2 text-[10px] flex-1 flex items-center justify-center gap-1.5 transition-all shadow-lg shadow-primary/20">
                          <Plus size={12} /> Create
                        </button>
                        <button onClick={() => setIsAdding(false)} className="bg-white/5 hover:bg-white/10 text-white/50 hover:text-white rounded-lg px-3 py-1.5 text-[10px] transition-all">
                          Cancel
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {modalStory && (
        <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-[100] p-6 animate-in fade-in duration-300">
          <div className="bg-[#1e1e1e] w-full max-w-2xl max-h-[85vh] rounded-2xl shadow-2xl overflow-hidden border border-border/40 flex flex-col animate-in zoom-in-95 duration-300">
            <div className="flex-shrink-0 p-5 border-b border-border/40 flex items-center justify-between bg-white/[0.02]">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-primary/10 rounded-xl text-primary">
                  <Bot size={20} />
                </div>
                <h2 className="text-sm font-bold tracking-tight">Story Details: <span className="font-mono text-primary/80 opacity-80">{modalStory.id}</span></h2>
              </div>
              <button 
                onClick={() => { setActiveModalStory(null); setAnswers({}); }}
                className="p-2 hover:bg-white/5 text-text-muted hover:text-white rounded-xl transition-all"
              >
                <Plus size={18} className="rotate-45" />
              </button>
            </div>
            
            <div className="flex-1 overflow-y-auto p-6 md:p-8 space-y-8 custom-scrollbar">
              <div className="space-y-3">
                <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Title</label>
                <div className="text-base font-bold p-4 bg-black/20 rounded-xl border border-white/5 tracking-tight leading-relaxed">{modalStory.title}</div>
              </div>
              
              <div className="flex flex-wrap gap-8">
                 <div className="space-y-3">
                   <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Status</label>
                   <div className="text-[10px] font-bold px-3 py-1 bg-primary/10 text-primary rounded-full border border-primary/20 w-fit uppercase tracking-widest">{modalStory.status}</div>
                 </div>
                 {modalStory.agent && (
                   <div className="space-y-3">
                     <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Assigned Agent</label>
                     <div className="flex items-center gap-2 text-xs font-bold text-white/90">
                        <div className="w-1.5 h-1.5 bg-primary rounded-full animate-pulse" />
                        {modalStory.agent}
                     </div>
                   </div>
                 )}
              </div>

              <div className="space-y-3">
                <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">
                   {modalStory.status === 'Clarification Required' ? 'AI Assessment' : 'Story Mission & Criteria'}
                </label>
                <div className="text-[13px] p-5 bg-black/30 rounded-xl border border-white/5 whitespace-pre-wrap font-sans text-white/80 leading-[1.7] shadow-inner">
                  {modalStory.status === 'Clarification Required' 
                    ? modalStory.description?.split(/Clarifying Questions:?/i)[0] || modalStory.description
                    : modalStory.description || "No mission requirements defined yet."}
                </div>
              </div>

              {modalStory.reviewer_feedback && (
                <div className="space-y-3">
                  <label className="text-[9px] uppercase font-bold text-blue-400 tracking-[0.2em] opacity-80">Latest Audit Activity & Progress</label>
                  <div className="text-[12px] p-5 bg-blue-500/[0.03] rounded-xl border border-blue-500/20 whitespace-pre-wrap font-mono text-blue-200/70 leading-[1.6]">
                    {modalStory.reviewer_feedback.trim()}
                  </div>
                </div>
              )}
              
              {modalStory.status === 'Clarification Required' && (
                <div className="space-y-4 pt-4 border-t border-border/40">
                  <label className="text-[9px] uppercase font-bold text-primary tracking-[0.2em] block">Clarifying Questions & Your Answers</label>
                  {questions && questions.length > 0 ? (
                    questions.map((q, i) => (
                      <div key={i} className="space-y-2 group">
                        <p className="text-xs text-text group-hover:text-primary transition-colors font-medium ml-1">Q: {q}</p>
                        <textarea 
                            placeholder="Your answer..."
                            value={answers[q] || ''}
                            onChange={(e) => setAnswers(prev => ({ ...prev, [q]: e.target.value }))}
                            className="w-full bg-black/40 border border-white/5 rounded-xl p-4 text-sm text-text outline-none focus:border-primary/50 transition-all h-24"
                        />
                      </div>
                    ))
                  ) : (
                    <textarea 
                      autoFocus
                      placeholder="Provide more details..."
                      value={answers['default'] || ''}
                      onChange={(e) => setAnswers(prev => ({ ...prev, 'default': e.target.value }))}
                      className="w-full h-32 bg-black/40 border border-white/5 rounded-xl p-4 text-sm text-text outline-none focus:border-primary/50 transition-all"
                    />
                  )}
                </div>
              )}
            </div>
            
            <div className="p-5 border-t border-border/40 flex justify-end gap-3 bg-white/[0.01]">
              <button 
                onClick={() => { setActiveModalStory(null); setAnswers({}); }}
                className="px-5 py-2 rounded-xl text-xs font-bold text-text-muted hover:text-white hover:bg-white/5 transition-all"
              >
                Close
              </button>
              {modalStory.status === 'Clarification Required' && (
                <button 
                  onClick={handleAnswerQuestions}
                  disabled={Object.values(answers).every(v => !v.trim())}
                  className="px-6 py-2 bg-primary hover:bg-primary/90 disabled:opacity-30 disabled:grayscale text-white rounded-xl text-xs font-bold shadow-lg shadow-primary/20 transition-all active:scale-95"
                >
                  Submit Answers
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
