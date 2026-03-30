import { useState, useEffect } from 'react';
import { Editor } from '@monaco-editor/react';
import { invoke } from '@tauri-apps/api/core';
import { Save, Loader2, CheckCircle, AlertCircle } from 'lucide-react';

export function CodeEditor({ path }: { path: string }) {
  const [content, setContent] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<'idle' | 'success' | 'error'>('idle');

  useEffect(() => {
    loadFile();
  }, [path]);

  const loadFile = async () => {
    setLoading(true);
    setStatus('idle');
    try {
      const data = await invoke<string>('read_file', { path });
      setContent(data);
    } catch (err) {
      console.error("Failed to read file:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await invoke('write_file', { path, content });
      setStatus('success');
      setTimeout(() => setStatus('idle'), 2000);
    } catch (err) {
      console.error("Failed to write file:", err);
      setStatus('error');
    } finally {
      setSaving(false);
    }
  };

  const getLanguage = (p: string) => {
    const ext = p.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx': return 'typescript';
      case 'js':
      case 'jsx': return 'javascript';
      case 'rs': return 'rust';
      case 'json': return 'json';
      case 'css': return 'css';
      case 'html': return 'html';
      case 'md': return 'markdown';
      default: return 'plaintext';
    }
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center bg-background text-text-muted">
        <Loader2 className="animate-spin mr-2" />
        Loading {path.split('/').pop()}...
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-[#1e1e1e]">
      {/* Editor Toolbar */}
      <div className="h-10 bg-surface flex items-center justify-between px-4 border-b border-border">
        <div className="flex items-center gap-3">
          <span className="text-[10px] font-mono text-text-muted opacity-50 uppercase tracking-widest">Editing</span>
          <span className="text-xs font-bold text-white/90">{path.split('/').pop()}</span>
        </div>
        
        <div className="flex items-center gap-2">
          {status === 'success' && <div className="text-green-500 flex items-center gap-1 text-[10px] font-bold uppercase"><CheckCircle size={12} /> Saved</div>}
          {status === 'error' && <div className="text-red-500 flex items-center gap-1 text-[10px] font-bold uppercase"><AlertCircle size={12} /> Error Saving</div>}
          
          <button 
            onClick={handleSave}
            disabled={saving}
            className={`flex items-center gap-2 px-3 py-1 rounded bg-primary hover:bg-primary/90 text-white text-[10px] font-bold uppercase transition-all shadow-lg shadow-primary/20 active:scale-95 disabled:opacity-50`}
          >
            {saving ? <Loader2 size={12} className="animate-spin" /> : <Save size={12} />}
            Save
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        <Editor
          height="100%"
          language={getLanguage(path)}
          theme="vs-dark"
          value={content}
          onChange={(val) => setContent(val || '')}
          options={{
            fontSize: 13,
            fontFamily: "'Fira Code', 'JetBrains Mono', monospace",
            minimap: { enabled: true },
            scrollBeyondLastLine: false,
            automaticLayout: true,
            padding: { top: 16, bottom: 16 },
            lineNumbersMinChars: 3,
          }}
        />
      </div>
    </div>
  );
}
