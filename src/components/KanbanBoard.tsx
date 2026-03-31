import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Play, Pause } from 'lucide-react';
import { KanbanColumn } from './kanban/KanbanColumn';
import { StoryModal } from './kanban/StoryModal';

interface Story {
  id: string;
  title: string;
  description?: string;
  reviewer_feedback?: string;
  status: string;
  ai_ready: number;
  ai_hold: number;
  skip_clarification: number;
  agent?: string;
  state?: string;
}

interface StoryTask {
  id: number;
  title: string;
  completed: boolean;
}

const COLUMNS = [
  'Raw Requirements',
  'Clarification Required',
  'Backlog',
  'To Do',
  'In Progress',
  'Review',
  'Testing',
  'Documentation',
  'Done'
];

export function KanbanBoard() {
  const [stories, setStories] = useState<Story[]>([]);
  const [activeModalStory, setActiveModalStory] = useState<Story | null>(null);
  const activeModalStoryRef = useRef<Story | null>(null);
  const [storyTasks, setStoryTasks] = useState<Record<string, StoryTask[]>>({});
  const [isAdding, setIsAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [skipClarification, setSkipClarification] = useState(false);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [activeAIColumns, setActiveAIColumns] = useState<Set<string>>(new Set());
  const [copyFeedback, setCopyFeedback] = useState<string | null>(null);

  useEffect(() => {
    activeModalStoryRef.current = activeModalStory;
  }, [activeModalStory]);

  const fetchStories = async () => {
    try {
      const data: Story[] = await invoke('get_stories');
      setStories(data);
      data.forEach(s => fetchTasksForStory(s.id));
    } catch (e) {
      console.error(e);
    }
  };

  const fetchTasksForStory = async (id: string) => {
    try {
      const tasks: StoryTask[] = await invoke('get_story_tasks', { storyId: id });
      setStoryTasks(prev => ({ ...prev, [id]: tasks }));
      if (activeModalStoryRef.current?.id === id) {
        setActiveModalStory(prev => prev ? { ...prev } : null);
      }
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    fetchStories();
    const unlistenRefresh = listen('refresh_board', () => {
      fetchStories();
    });
    return () => {
      unlistenRefresh.then(u => u());
    };
  }, []);

  useEffect(() => {
    // Auto-dispatch idle stories in active columns
    const idleStories = stories.filter(s => 
      activeAIColumns.has(s.status) && 
      s.ai_hold === 0 && 
      s.state === 'idle'
    );
    idleStories.forEach(s => handleDispatch(s.id));
  }, [stories, activeAIColumns]);

  const handleDragStart = (e: React.DragEvent, id: string) => {
    setDraggingId(id);
    e.dataTransfer.setData('storyId', id);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = async (e: React.DragEvent, status: string) => {
    e.preventDefault();
    const id = e.dataTransfer.getData('storyId');
    setDraggingId(null);
    try {
      await invoke('update_story_status', { id, status });
      fetchStories();
    } catch (e) {
      console.error(e);
    }
  };

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    try {
      const story: Story = await invoke('create_story', { title: newTitle, status: 'Raw Requirements' });
      if (skipClarification) {
        await invoke('update_story_skip_clarification', { id: story.id, skip: true });
      }
      setIsAdding(false);
      setNewTitle('');
      setSkipClarification(false);
      fetchStories();
      handleDispatch(story.id);
    } catch (e) {
      console.error(e);
    }
  };

  const handleDispatch = async (id: string, context?: string) => {
    try {
      await invoke('dispatch_story', { id, additionalContext: context });
      fetchStories();
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleHold = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke('toggle_story_hold', { id });
      fetchStories();
    } catch (e) {
      console.error(e);
    }
  };

  const handleUpdateSkipClarification = async (id: string, skip: boolean) => {
    try {
      await invoke('update_story_skip_clarification', { id, skip });
      fetchStories();
      if (activeModalStory?.id === id) {
        setActiveModalStory(prev => prev ? { ...prev, skip_clarification: skip ? 1 : 0 } : null);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm('Delete this story?')) return;
    try {
      await invoke('delete_story', { id });
      fetchStories();
    } catch (e) {
      console.error(e);
    }
  };

  const handleCopy = (text: string, type: string) => {
    navigator.clipboard.writeText(text);
    setCopyFeedback(type);
    setTimeout(() => setCopyFeedback(null), 2000);
  };

  const handleAnswerQuestions = (answers: Record<string, string>) => {
    const formattedAnswers = Object.entries(answers)
      .map(([q, a]) => `**Q: ${q}**\n**A: ${a}**`)
      .join('\n\n');
    handleDispatch(activeModalStory!.id, formattedAnswers);
    setActiveModalStory(null);
  };

  const toggleAIForColumn = async (col: string) => {
     setActiveAIColumns(prev => {
        const next = new Set(prev);
        if (next.has(col)) {
           next.delete(col);
           invoke('set_column_ai_paused', { status: col, paused: true }).then(() => fetchStories());
        } else {
           next.add(col);
           invoke('set_column_ai_paused', { status: col, paused: false }).then(() => {
              const colStories = stories.filter(s => s.status === col && s.ai_hold === 0);
              colStories.forEach(s => handleDispatch(s.id));
              fetchStories();
           });
        }
        return next;
     });
  };

  const pauseAll = () => {
    COLUMNS.forEach(col => {
      if (activeAIColumns.has(col)) toggleAIForColumn(col);
    });
  };

  const resumeAll = () => {
    COLUMNS.forEach(col => {
      if (!activeAIColumns.has(col)) toggleAIForColumn(col);
    });
  };

  return (
    <div className="flex-1 flex flex-col min-w-0 bg-background/30 backdrop-blur-3xl p-6 overflow-hidden">
      <div className="flex-shrink-0 flex items-center justify-between mb-8 px-2">
        <div>
          <h1 className="text-2xl font-black italic tracking-tighter text-white flex items-center gap-2 group cursor-default">
            SANDMAN <span className="text-primary not-italic tracking-widest text-[10px] font-bold bg-primary/10 px-2 py-0.5 rounded-full border border-primary/20 group-hover:shadow-[0_0_15px_rgba(59,130,246,0.3)] transition-all">IDE</span>
          </h1>
          <p className="text-[10px] uppercase font-bold tracking-[0.3em] text-[#555] mt-1 ml-0.5">Autonomous SDLC Pipeline</p>
        </div>

        <div className="flex gap-2">
          {activeAIColumns.size > 0 ? (
            <button 
              onClick={pauseAll}
              className="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-500 border border-red-500/30 rounded-lg text-xs font-bold uppercase tracking-wider flex items-center gap-2 transition-all"
            >
              <Pause size={14} fill="currentColor" /> Pause AI
            </button>
          ) : (
            <button 
              onClick={resumeAll}
              className="px-4 py-2 bg-primary/10 hover:bg-primary/20 text-primary border border-primary/30 rounded-lg text-xs font-bold uppercase tracking-wider flex items-center gap-2 transition-all"
            >
              <Play size={14} fill="currentColor" /> Resume AI
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-x-auto custom-scrollbar pb-4 -mx-6 px-6">
        <div className="flex gap-4 h-full min-w-min">
          {COLUMNS.map((col) => (
            <KanbanColumn
              key={col}
              status={col}
              stories={stories.filter(s => s.status === col)}
              storyTasks={storyTasks}
              activeAIColumns={activeAIColumns}
              onToggleAI={toggleAIForColumn}
              onAddStory={() => setIsAdding(true)}
              onOpenStory={(s) => { setActiveModalStory(s); fetchTasksForStory(s.id); }}
              onDragStart={handleDragStart}
              onDragOver={handleDragOver}
              onDrop={handleDrop}
              onDeleteStory={handleDelete}
              onToggleHold={handleToggleHold}
              draggingId={draggingId}
              isAdding={isAdding && col === 'Raw Requirements'}
              newTitle={newTitle}
              setNewTitle={setNewTitle}
              skipClarification={skipClarification}
              setSkipClarification={setSkipClarification}
              onCreateStory={handleCreate}
              onCancelAdd={() => setIsAdding(false)}
            />
          ))}
        </div>
      </div>

      {activeModalStory && (
        <StoryModal
          story={activeModalStory}
          tasks={storyTasks[activeModalStory.id] || []}
          onClose={() => setActiveModalStory(null)}
          onCopy={handleCopy}
          copyFeedback={copyFeedback}
          onUpdateSkipClarification={handleUpdateSkipClarification}
          onAnswerQuestions={handleAnswerQuestions}
        />
      )}
    </div>
  );
}
