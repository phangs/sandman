import { useState, useEffect } from 'react';
import { Save, Cpu, Globe, Key, CheckCircle2, Circle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  endpoint: string;
  active: boolean;
  model: string;
}

interface Config {
  providers: ProviderConfig[];
  selected_model_group: string;
  column_strategies: Record<string, string>;
}

const COLUMNS = ["Raw Requirements", "Backlog", "To Do", "In Progress", "Review", "Testing", "Documentation"];

export function SettingsView() {
  const [config, setConfig] = useState<Config | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const result = await invoke<Config>('get_config');
      setConfig(result);
    } catch (err) {
      console.error("Failed to load config:", err);
    }
  };

  const handleToggle = (id: string) => {
    if (!config) return;
    setConfig({
      ...config,
      providers: config.providers.map(p => 
        p.id === id ? { ...p, active: !p.active } : p
      )
    });
  };

  const handleKeyChange = (id: string, key: string) => {
    if (!config) return;
    setConfig({
      ...config,
      providers: config.providers.map(p => 
        p.id === id ? { ...p, api_key: key } : p
      )
    });
  };

  const handleModelChange = (id: string, model: string) => {
    if (!config) return;
    setConfig({
      ...config,
      providers: config.providers.map(p => 
        p.id === id ? { ...p, model } : p
      )
    });
  };

  const handleStrategyChange = (column: string, providerId: string) => {
    if (!config) return;
    setConfig({
      ...config,
      column_strategies: {
        ...config.column_strategies,
        [column]: providerId
      }
    });
  };

  const save = async () => {
    if (!config) return;
    setIsSaving(true);
    try {
      await invoke('save_global_config', { config });
      setIsSaving(false);
    } catch (err) {
      console.error("Save failed:", err);
      setIsSaving(false);
    }
  };

  if (!config) return <div className="p-8 text-text-muted">Loading settings...</div>;

  return (
    <div className="h-full bg-background overflow-y-auto p-8 max-w-5xl mx-auto pb-20">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-text mb-2 tracking-tight">AI Settings & Workflows</h1>
          <p className="text-text-muted">Route specific SDLC stages to your preferred LLM providers.</p>
        </div>
        <button 
          onClick={save}
          disabled={isSaving}
          className="bg-primary hover:bg-primary/90 text-white px-6 py-2.5 rounded-lg flex items-center gap-2 font-bold transition-all shadow-lg shadow-primary/20"
        >
          <Save size={18} /> {isSaving ? 'Saving...' : 'Deploy Configuration'}
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        
        {/* Left: Provider Configuration */}
        <div className="lg:col-span-2 space-y-6">
          <h2 className="text-xs font-bold text-text-muted uppercase tracking-[0.2em] mb-4">LLM Providers</h2>
          {config.providers.map((provider) => (
            <div key={provider.id} className={`border rounded-xl p-6 bg-surface transition-all duration-300 ${provider.active ? 'border-primary/40 ring-1 ring-primary/10 shadow-xl shadow-primary/5' : 'border-border'}`}>
              <div className="flex items-center justify-between mb-6">
                <div className="flex items-center gap-4">
                  <div className={`p-3 rounded-lg ${provider.active ? 'bg-primary/10 text-primary' : 'bg-[#1e1e1e] text-text-muted'}`}>
                    {provider.id === 'ollama' ? <Cpu size={24} /> : <Globe size={24} />}
                  </div>
                  <div>
                    <h3 className="text-lg font-bold text-text mb-0.5">{provider.name}</h3>
                    <code className="text-[10px] bg-black/30 px-1.5 py-0.5 rounded text-text-muted">{provider.endpoint}</code>
                  </div>
                </div>
                <button 
                  onClick={() => handleToggle(provider.id)}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-bold uppercase tracking-wider transition-all ${provider.active ? 'bg-primary/20 text-primary' : 'bg-[#1e1e1e] text-text-muted hover:text-text hover:bg-white/5'}`}
                >
                  {provider.active ? <CheckCircle2 size={16} /> : <Circle size={16} />}
                  {provider.active ? 'Active' : 'Enable'}
                </button>
              </div>

              {provider.active && (
                <div className="space-y-5 animate-in fade-in slide-in-from-top-2 duration-400">
                  <div className="grid grid-cols-2 gap-6">
                    <div className="space-y-2">
                      <label className="text-[10px] font-bold text-text-muted uppercase tracking-widest flex items-center gap-2 opacity-70">
                         <Cpu size={12} /> Model / Engine
                      </label>
                      <input 
                        type="text"
                        placeholder="e.g. gpt-4o, claude-3-5-sonnet"
                        value={provider.model || ''}
                        onChange={(e) => handleModelChange(provider.id, e.target.value)}
                        className="w-full bg-[#1e1e1e] border border-border/60 rounded-lg px-4 py-2.5 text-sm text-text focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-mono"
                      />
                    </div>

                    {provider.id !== 'ollama' && (
                      <div className="space-y-2">
                        <label className="text-[10px] font-bold text-text-muted uppercase tracking-widest flex items-center gap-2 opacity-70">
                          <Key size={12} /> Secret Key
                        </label>
                        <input 
                          type="password"
                          placeholder="••••••••••••••••"
                          value={provider.api_key || ''}
                          onChange={(e) => handleKeyChange(provider.id, e.target.value)}
                          className="w-full bg-[#1e1e1e] border border-border/60 rounded-lg px-4 py-2.5 text-sm text-text focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-mono"
                        />
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Right: Column Strategies */}
        <div className="space-y-6">
           <h2 className="text-xs font-bold text-text-muted uppercase tracking-[0.2em] mb-4">SDLC Routing Rules</h2>
           <div className="bg-surface border border-border rounded-xl p-6 space-y-6 shadow-xl shadow-black/20">
              <div className="p-3 bg-primary/5 rounded-lg border border-primary/10 text-[11px] text-text-muted leading-relaxed">
                Assign specific LLMs to different story stages to optimize for speed, cost, or reasoning depth.
              </div>
              
              {COLUMNS.map(col => (
                <div key={col} className="space-y-2.5">
                  <label className="text-[10px] font-black text-text-muted uppercase tracking-widest block">{col}</label>
                  <select 
                    value={config.column_strategies[col] || "ollama"}
                    onChange={(e) => handleStrategyChange(col, e.target.value)}
                    className="w-full bg-[#1e1e1e] border border-border/60 rounded-lg px-3 py-2 text-xs text-text outline-none focus:border-primary focus:ring-1 focus:ring-primary transition-all appearance-none cursor-pointer"
                  >
                    {config.providers.map(p => (
                      <option key={p.id} value={p.id}>
                        {p.name} {p.active ? "" : "(Inactive)"}
                      </option>
                    ))}
                  </select>
                </div>
              ))}
           </div>

           <div className="p-6 bg-surface border border-border rounded-xl">
             <h3 className="text-xs font-bold text-text mb-3 uppercase tracking-widest text-text-muted">Security & Privacy</h3>
             <p className="text-[10px] text-text-muted leading-relaxed opacity-60">
               Sandman stores your API keys locally in an encrypted vault. No telemetry or credentials are sent to Sandman servers.
             </p>
           </div>
        </div>
      </div>
    </div>
  );
}
