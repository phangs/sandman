import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { FileText, Save, Trash2, Brain, Workflow, ClipboardList, ChevronDown, ChevronRight } from 'lucide-react';

interface Artifact {
  id: string;
  story_id: string;
  name: string;
  content: string;
  a_type: string;
  created_at: number;
  updated_at: number;
  saved: number;
}

interface ArtifactsListProps {
  storyId: string;
}

export const ArtifactsList: React.FC<ArtifactsListProps> = ({ storyId }) => {
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchArtifacts = async () => {
    try {
      const data = await invoke<Artifact[]>('get_artifacts', { storyId });
      setArtifacts(data);
    } catch (err) {
      console.error("Failed to fetch artifacts:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchArtifacts();
    
    const unlisten = listen('refresh_artifacts', () => {
      fetchArtifacts();
    });

    return () => {
      unlisten.then(f => f());
    };
  }, [storyId]);

  const toggleSave = async (id: string) => {
    try {
      await invoke('toggle_artifact_save', { id });
      fetchArtifacts();
    } catch (err) {
      console.error("Failed to toggle save:", err);
    }
  };

  const purge = async () => {
    if (!window.confirm("Purge all unsaved artifacts for this story?")) return;
    try {
      await invoke('purge_artifacts', { storyId });
      fetchArtifacts();
    } catch (err) {
      console.error("Failed to purge artifacts:", err);
    }
  };

  const getIcon = (type: string) => {
    switch (type) {
      case 'brainstorm': return <Brain size={14} className="text-purple-400" />;
      case 'workflow': return <Workflow size={14} className="text-blue-400" />;
      case 'plan': return <ClipboardList size={14} className="text-green-400" />;
      default: return <FileText size={14} className="text-gray-400" />;
    }
  };

  if (loading) return <div className="text-[10px] text-text-muted animate-pulse">Loading artifacts...</div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <label className="text-[9px] uppercase font-bold text-text-muted tracking-[0.2em] opacity-60">AI Artifacts & Scratchpads</label>
        {artifacts.some(a => a.saved === 0) && (
          <button 
            onClick={purge}
            className="flex items-center gap-1.5 text-[9px] uppercase font-bold text-red-400 hover:text-red-500 transition-colors cursor-pointer"
          >
            <Trash2 size={10} />
            Purge Temp
          </button>
        )}
      </div>

      {artifacts.length === 0 ? (
        <div className="text-[11px] text-text-muted italic border border-dashed border-white/5 rounded-xl p-4 text-center">
          No artifacts generated yet. The agent creates these during brainstorming and planning.
        </div>
      ) : (
        <div className="space-y-2">
          {artifacts.map((a) => (
            <div key={a.id} className={`border border-white/5 rounded-xl overflow-hidden transition-all ${expandedId === a.id ? 'bg-black/30 ring-1 ring-primary/20' : 'bg-black/20 hover:bg-black/25'}`}>
              <div 
                className="p-3 flex items-center justify-between cursor-pointer group"
                onClick={() => setExpandedId(expandedId === a.id ? null : a.id)}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="shrink-0 p-1.5 bg-white/5 rounded-lg group-hover:bg-white/10 transition-colors">
                    {getIcon(a.a_type)}
                  </div>
                  <div className="min-w-0 flex flex-col">
                    <span className="text-[11px] font-bold text-white/90 truncate tracking-tight">{a.name}</span>
                    <span className="text-[9px] text-text-muted uppercase tracking-widest font-medium opacity-60">{a.a_type}</span>
                  </div>
                </div>
                
                <div className="flex items-center gap-2">
                   <button 
                    onClick={(e) => { e.stopPropagation(); toggleSave(a.id); }}
                    title={a.saved === 1 ? "Click to unsave (make temporary)" : "Save artifact permanently"}
                    className={`p-1.5 rounded-lg transition-all ${a.saved === 1 ? 'text-primary bg-primary/10 border border-primary/20' : 'text-text-muted hover:text-white bg-white/5 hover:bg-white/10 border border-transparent'}`}
                  >
                    <Save size={12} />
                  </button>
                  {expandedId === a.id ? <ChevronDown size={14} className="text-text-muted" /> : <ChevronRight size={14} className="text-text-muted" />}
                </div>
              </div>

              {expandedId === a.id && (
                <div className="px-4 pb-4 pt-0 animate-in slide-in-from-top-1 duration-200">
                  <div className="p-3 bg-black/40 rounded-lg border border-white/5 text-[12px] text-white/80 whitespace-pre-wrap font-mono leading-relaxed max-h-[300px] overflow-y-auto custom-scrollbar shadow-inner">
                    {a.content}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
