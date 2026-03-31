import React from 'react';
import { Bot, CheckCircle, Copy, Plus, TerminalSquare } from 'lucide-react';
import { ArtifactsList } from '../ArtifactsList';

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

interface StoryModalProps {
  story: Story;
  tasks: StoryTask[];
  onClose: () => void;
  onCopy: (text: string, type: string) => void;
  copyFeedback: string | null;
  onUpdateSkipClarification: (id: string, skip: boolean) => void;
  onAnswerQuestions: (answers: Record<string, string>) => void;
}

export function StoryModal({ 
  story, 
  tasks, 
  onClose, 
  onCopy, 
  copyFeedback,
  onUpdateSkipClarification,
  onAnswerQuestions
}: StoryModalProps) {
  const [answers, setAnswers] = React.useState<Record<string, string>>({});

  const questions = story.description 
    ? story.description
        .split(/Clarifying Questions:?/i)[1]
        ?.split('\n')
        .filter(l => l.trim().match(/^[-*•?]|^\d+\./))
        .map(l => l.trim())
    : [];

  const done = tasks.filter(t => t.completed).length;
  const pct = tasks.length > 0 ? Math.round((done / tasks.length) * 100) : 0;

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-[100] p-6 animate-in fade-in duration-300">
      <div className="bg-[#1e1e1e] w-full max-w-2xl max-h-[85vh] rounded-2xl shadow-2xl overflow-hidden border border-border/40 flex flex-col animate-in zoom-in-95 duration-300">
        <div className="flex-shrink-0 p-5 border-b border-border/40 flex items-center justify-between bg-white/[0.02]">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary/10 rounded-xl text-primary">
              <Bot size={20} />
            </div>
            <h2 className="text-sm font-bold tracking-tight">Story Details: <span className="font-mono text-primary/80 opacity-80">{story.id}</span></h2>
          </div>
          <button 
            onClick={onClose}
            className="p-2 hover:bg-white/5 text-text-muted hover:text-white rounded-xl transition-all"
          >
            <Plus size={18} className="rotate-45" />
          </button>
        </div>
        
        <div className="flex-1 overflow-y-auto p-6 md:p-8 space-y-8 custom-scrollbar">
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Title</label>
              <button 
                onClick={() => onCopy(story.title, 'title')}
                className="flex items-center gap-1.5 text-[9px] uppercase font-bold text-text-muted hover:text-primary transition-colors cursor-pointer"
              >
                {copyFeedback === 'title' ? <CheckCircle size={12} className="text-green-500" /> : <Copy size={12} />}
                {copyFeedback === 'title' ? 'Copied' : 'Copy'}
              </button>
            </div>
            <div className="text-base font-bold p-4 bg-black/20 rounded-xl border border-white/5 tracking-tight leading-relaxed">{story.title}</div>
          </div>
          
          <div className="flex flex-wrap gap-8">
             <div className="space-y-3">
               <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Status</label>
               <div className="text-[10px] font-bold px-3 py-1 bg-primary/10 text-primary rounded-full border border-primary/20 w-fit uppercase tracking-widest">{story.status}</div>
             </div>
             {story.agent && (
               <div className="space-y-3">
                 <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Assigned Agent</label>
                 <div className="flex items-center gap-2 text-xs font-bold text-white/90">
                    <div className="w-1.5 h-1.5 bg-primary rounded-full animate-pulse" />
                    {story.agent}
                 </div>
               </div>
             )}
             <div className="space-y-3">
                <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Control</label>
                <label className="flex items-center gap-3 cursor-pointer group select-none">
                    <div 
                       onClick={() => onUpdateSkipClarification(story.id, story.skip_clarification === 0)}
                       className={`w-8 h-4 rounded-full transition-all duration-300 relative ${story.skip_clarification === 1 ? 'bg-primary' : 'bg-white/10'}`}
                    >
                       <div className={`absolute top-0.5 w-3 h-3 bg-white rounded-full transition-all duration-300 ${story.skip_clarification === 1 ? 'left-4.5' : 'left-0.5'}`} />
                    </div>
                    <span className="text-[10px] text-text-muted group-hover:text-text transition-colors">Skip Clarification</span>
                </label>
              </div>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">
                 {story.status === 'Clarification Required' ? 'AI Assessment' : 'Story Mission & Criteria'}
              </label>
              <button 
                onClick={() => {
                  const textToCopy = story.status === 'Clarification Required' 
                    ? story.description?.split(/Clarifying Questions:?/i)[0] || story.description
                    : story.description || "No mission requirements defined yet.";
                  onCopy(textToCopy || '', 'desc');
                }}
                className="flex items-center gap-1.5 text-[9px] uppercase font-bold text-text-muted hover:text-primary transition-colors cursor-pointer"
              >
                {copyFeedback === 'desc' ? <CheckCircle size={12} className="text-green-500" /> : <Copy size={12} />}
                {copyFeedback === 'desc' ? 'Copied' : 'Copy'}
              </button>
            </div>
            <div className="text-[13px] p-5 bg-black/30 rounded-xl border border-white/5 whitespace-pre-wrap font-sans text-white/80 leading-[1.7] shadow-inner">
              {story.status === 'Clarification Required' 
                ? story.description?.split(/Clarifying Questions:?/i)[0] || story.description
                : story.description || "No mission requirements defined yet."}
            </div>
          </div>

          <ArtifactsList storyId={story.id} />

          {story.reviewer_feedback && (
            <div className="space-y-3">
              <label className="text-[9px] uppercase font-bold text-blue-400 tracking-[0.2em] opacity-80">Latest Audit Activity & Progress</label>
              <div className="text-[12px] p-5 bg-blue-500/[0.03] rounded-xl border border-blue-500/20 whitespace-pre-wrap font-mono text-blue-200/70 leading-[1.6]">
                {story.reviewer_feedback.trim()}
              </div>
            </div>
          )}

          {tasks.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">Task Breakdown</label>
                <div className="flex items-center gap-2">
                  <span className="text-[9px] font-bold font-mono text-text-muted">{done}/{tasks.length} done</span>
                  <span className={`text-[9px] font-bold font-mono px-2 py-0.5 rounded-full border ${
                    pct === 100 ? 'bg-green-500/10 border-green-500/20 text-green-400' :
                    pct > 50 ? 'bg-primary/10 border-primary/20 text-primary' :
                    'bg-yellow-500/10 border-yellow-500/20 text-yellow-400'
                  }`}>{pct}%</span>
                </div>
              </div>
              <div className="h-1.5 w-full bg-white/5 rounded-full overflow-hidden mb-1">
                <div
                  className={`h-full rounded-full transition-all duration-700 ${
                    pct === 100 ? 'bg-green-500' : pct > 50 ? 'bg-primary' : 'bg-yellow-500/80'
                  }`}
                  style={{ width: `${pct}%` }}
                />
              </div>
              <div className="bg-black/30 rounded-xl border border-white/5 overflow-hidden">
                <table className="w-full text-[11px]">
                  <thead>
                    <tr className="border-b border-white/5">
                      <th className="text-left px-4 py-2.5 text-[9px] uppercase tracking-widest font-bold text-text-muted opacity-50 w-8">#</th>
                      <th className="text-left px-4 py-2.5 text-[9px] uppercase tracking-widest font-bold text-text-muted opacity-50">Task Description</th>
                      <th className="text-right px-4 py-2.5 text-[9px] uppercase tracking-widest font-bold text-text-muted opacity-50 w-24">Status</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-white/5">
                    {tasks.map((task, idx) => (
                      <tr key={task.id} className="hover:bg-white/[0.02] transition-colors group">
                        <td className="px-4 py-3 font-mono opacity-30">{idx + 1}</td>
                        <td className={`px-4 py-3 font-medium ${task.completed ? 'text-text-muted line-through opacity-50' : 'text-white/90'}`}>{task.title}</td>
                        <td className="px-4 py-3 text-right">
                          <div className={`text-[9px] font-bold uppercase transition-all ${task.completed ? 'text-green-400' : 'text-yellow-500/60'}`}>
                            {task.completed ? <CheckCircle size={12} className="ml-auto" /> : 'Pending'}
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {story.status === 'Clarification Required' && questions && questions.length > 0 && (
             <div className="mt-8 space-y-6 pt-8 border-t border-white/5">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-yellow-400/10 rounded-xl text-yellow-400">
                    <TerminalSquare size={18} />
                  </div>
                  <h3 className="text-sm font-bold tracking-tight">AI Clarification Needed</h3>
                </div>
                <div className="space-y-6">
                  {questions.map((q, i) => (
                    <div key={i} className="space-y-3 bg-black/20 p-5 rounded-2xl border border-white/5">
                      <label className="text-xs font-bold text-white/90 leading-relaxed block">{q}</label>
                      <textarea
                        className="w-full bg-black/40 border border-white/5 rounded-xl p-4 text-xs text-white/80 focus:border-primary/50 outline-none transition-all placeholder:text-white/10"
                        placeholder="Provide answer or context here..."
                        value={answers[q] || ''}
                        onChange={(e) => setAnswers({ ...answers, [q]: e.target.value })}
                        rows={3}
                      />
                    </div>
                  ))}
                  <button 
                    onClick={() => onAnswerQuestions(answers)}
                    className="w-full bg-primary hover:bg-primary/90 text-white font-bold py-4 rounded-xl text-xs transition-all shadow-lg shadow-primary/20 hover:scale-[1.01] active:scale-[0.99] flex items-center justify-center gap-2"
                  >
                    <Plus size={16} /> Submit Answers & Dispatch Agent
                  </button>
                </div>
             </div>
          )}
        </div>
      </div>
    </div>
  );
}
