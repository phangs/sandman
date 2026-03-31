import React from 'react';
import { AlertOctagon, CircleDashed, Play, Plus } from 'lucide-react';
import { StoryCard } from './StoryCard';

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

interface KanbanColumnProps {
  status: string;
  stories: Story[];
  storyTasks: Record<string, StoryTask[]>;
  activeAIColumns: Set<string>;
  onToggleAI: (status: string) => void;
  onAddStory: () => void;
  onOpenStory: (story: Story) => void;
  onDragStart: (e: React.DragEvent, id: string) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent, status: string) => void;
  onDeleteStory: (id: string, e: React.MouseEvent) => void;
  onToggleHold: (id: string, e: React.MouseEvent) => void;
  draggingId: string | null;
  isAdding: boolean;
  newTitle: string;
  setNewTitle: (t: string) => void;
  skipClarification: boolean;
  setSkipClarification: (s: boolean) => void;
  onCreateStory: () => void;
  onCancelAdd: () => void;
}

export function KanbanColumn({
  status,
  stories,
  storyTasks,
  activeAIColumns,
  onToggleAI,
  onAddStory,
  onOpenStory,
  onDragStart,
  onDragOver,
  onDrop,
  onDeleteStory,
  onToggleHold,
  draggingId,
  isAdding,
  newTitle,
  setNewTitle,
  skipClarification,
  setSkipClarification,
  onCreateStory,
  onCancelAdd
}: KanbanColumnProps) {
  return (
    <div
      onDragOver={onDragOver}
      onDrop={(e) => onDrop(e, status)}
      className="flex flex-col w-[300px] flex-shrink-0 bg-[#161616]/60 rounded-3xl p-1.5 border border-white/[0.03] shadow-2xl group/column select-none"
    >
      <div className="flex items-center justify-between p-3.5 mb-2 sticky top-0 bg-transparent z-10">
        <div className="flex items-center gap-2">
          <h2 className="text-xs font-bold uppercase tracking-widest text-[#999] group-hover/column:text-white/100 transition-colors pointer-events-none">{status}</h2>
          <span className="text-[10px] bg-white/5 border border-white/10 px-2 py-0.5 rounded-full font-mono text-text-muted">{stories.length}</span>
        </div>
        <div className="flex items-center gap-1">
           <button 
              onClick={() => onToggleAI(status)}
              className={`p-1.5 rounded-md transition-all hover:scale-110 active:scale-95 ${activeAIColumns.has(status) ? 'text-primary bg-primary/10 hover:bg-primary/20 shadow-[0_0_10px_rgba(59,130,246,0.2)]' : 'text-text-muted hover:text-white bg-white/5'}`}
           >
             {activeAIColumns.has(status) ? <AlertOctagon size={14} /> : <Play size={14} fill="currentColor" />}
           </button>
          {status === 'Raw Requirements' && (
            <button onClick={onAddStory} className="p-1.5 hover:bg-white/5 text-text-muted hover:text-white rounded-md transition-colors">
              <Plus size={14} />
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto custom-scrollbar flex flex-col gap-3 min-h-[100px]">
        {stories.length === 0 && !isAdding && (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 opacity-10 pointer-events-none border-2 border-dashed border-white/50 rounded-lg m-1">
            <CircleDashed size={16} />
            <span className="text-[10px] uppercase font-bold tracking-wider">Empty Column</span>
          </div>
        )}

        {stories.map((story) => (
          <StoryCard 
            key={story.id}
            story={story}
            tasks={storyTasks[story.id] || []}
            onOpen={() => onOpenStory(story)}
            onDragStart={onDragStart}
            onDelete={onDeleteStory}
            onToggleHold={onToggleHold}
            isDragging={draggingId === story.id}
          />
        ))}

        {status === 'Raw Requirements' && isAdding && (
          <div className="bg-[#1e1e1e] border border-primary/40 rounded-xl p-3 flex flex-col gap-3 shadow-2xl">
            <textarea
              autoFocus
              rows={6}
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              placeholder="Paste raw requirements here... (Ctrl+Enter to create)"
              className="bg-black/40 text-xs text-text border border-white/5 outline-none p-3 rounded-lg focus:border-primary/50 transition-all font-medium resize-none leading-relaxed"
            />
            <label className="flex items-center gap-3 px-1 cursor-pointer group select-none">
              <div 
                onClick={() => setSkipClarification(!skipClarification)}
                className={`w-8 h-4 rounded-full transition-all duration-300 relative ${skipClarification ? 'bg-primary' : 'bg-white/10'}`}
              >
                <div className={`absolute top-0.5 w-3 h-3 bg-white rounded-full transition-all duration-300 ${skipClarification ? 'left-4.5' : 'left-0.5'}`} />
              </div>
              <span className="text-[10px] text-text-muted group-hover:text-text transition-colors">Skip Clarify</span>
            </label>
            <div className="flex gap-2">
              <button onClick={onCreateStory} className="bg-primary hover:bg-primary/90 text-white font-bold rounded-lg px-4 py-2 text-[10px] flex-1">Create</button>
              <button onClick={onCancelAdd} className="bg-white/5 hover:bg-white/10 text-white/50 rounded-lg px-3 py-1.5 text-[10px]">Cancel</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
