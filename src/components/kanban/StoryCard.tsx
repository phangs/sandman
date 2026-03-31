import React from 'react';
import { AlertOctagon, CheckCircle, Play, TerminalSquare, Trash2 } from 'lucide-react';

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

interface StoryCardProps {
  story: Story;
  tasks: StoryTask[];
  onOpen: () => void;
  onDragStart: (e: React.DragEvent, id: string) => void;
  onDelete: (id: string, e: React.MouseEvent) => void;
  onToggleHold: (id: string, e: React.MouseEvent) => void;
  isDragging: boolean;
}

export function StoryCard({ 
  story, 
  tasks, 
  onOpen, 
  onDragStart, 
  onDelete, 
  onToggleHold,
  isDragging 
}: StoryCardProps) {
  const done = tasks.filter(t => t.completed).length;
  const pct = tasks.length > 0 ? Math.round((done / tasks.length) * 100) : 0;

  return (
    <div
      draggable
      onDragStart={(e) => onDragStart(e, story.id)}
      onClick={onOpen}
      className={`bg-[#1e1e1e]/80 border border-border/40 rounded-xl p-3 cursor-pointer active:cursor-grabbing shadow-lg hover:border-primary/50 hover:shadow-primary/5 hover:translate-y-[-1px] active:translate-y-[1px] transition-all duration-200 group/card relative
        ${isDragging ? 'opacity-50 ring-2 ring-primary border-transparent' : 'border-border/40'}
        ${story.state === 'failed' ? 'border-red-500/40 bg-red-900/5 shadow-red-500/5' : ''}
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
             onClick={(e) => onToggleHold(story.id, e)}
             className={`text-[9px] font-bold flex items-center gap-1.5 px-2 py-0.5 rounded-full border border-border transition-all ${story.ai_hold === 1 ? 'bg-background hover:bg-white/5 text-text-muted' : 'bg-green-500/10 border-green-500/20 text-green-400'}`}
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
              onClick={(e) => onDelete(story.id, e)}
              className="w-5 h-5 flex items-center justify-center rounded-full border border-border text-text-muted opacity-0 group-hover/card:opacity-100 hover:bg-red-500/20 hover:text-red-400 hover:border-red-500/30 transition-all"
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
         {tasks.length > 0 && <span className="text-[9px] font-bold font-mono text-text-muted">{pct}%</span>}
      </div>

      {tasks.length > 0 && (
        <div className="h-1 w-full bg-white/5 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-500 ${pct === 100 ? 'bg-green-500' : pct > 50 ? 'bg-primary' : 'bg-yellow-500/80'}`}
            style={{ width: `${pct}%` }}
          />
        </div>
      )}
    </div>
  );
}
