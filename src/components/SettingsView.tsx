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
}

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
    <div className="h-full bg-background overflow-y-auto p-8 max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-text mb-2 tracking-tight">AI Settings</h1>
          <p className="text-text-muted">Configure your model providers and API credentials.</p>
        </div>
        <button 
          onClick={save}
          disabled={isSaving}
          className="bg-primary hover:bg-primary/90 text-white px-4 py-2 rounded flex items-center gap-2 font-medium transition-colors"
        >
          <Save size={18} /> {isSaving ? 'Saving...' : 'Save Configuration'}
        </button>
      </div>

      <div className="space-y-6">
        {config.providers.map((provider) => (
          <div key={provider.id} className={`border rounded-lg p-6 bg-surface transition-colors ${provider.active ? 'border-primary/50' : 'border-border'}`}>
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className={`p-2 rounded ${provider.id === 'ollama' ? 'bg-blue-500/10 text-blue-400' : 'bg-green-500/10 text-green-400'}`}>
                  {provider.id === 'ollama' ? <Cpu size={24} /> : <Globe size={24} />}
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-text leading-none mb-1">{provider.name}</h3>
                  <span className="text-xs text-text-muted">{provider.endpoint}</span>
                </div>
              </div>
              <button 
                onClick={() => handleToggle(provider.id)}
                className={`flex items-center gap-2 text-sm font-medium transition-colors ${provider.active ? 'text-primary' : 'text-text-muted hover:text-text'}`}
              >
                {provider.active ? <CheckCircle2 size={20} /> : <Circle size={20} />}
                {provider.active ? 'Enabled' : 'Disabled'}
              </button>
            </div>

            {provider.active && (
              <div className="space-y-4 animate-in fade-in slide-in-from-top-2 duration-300">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <label className="text-xs font-semibold text-text-muted uppercase tracking-wider flex items-center gap-2">
                       <Cpu size={12} /> Model Name
                    </label>
                    <input 
                      type="text"
                      placeholder={provider.id === 'ollama' ? 'llama3' : 'gpt-4o'}
                      value={provider.model || ''}
                      onChange={(e) => handleModelChange(provider.id, e.target.value)}
                      className="w-full bg-[#1e1e1e] border border-border rounded px-3 py-2 text-sm text-text focus:border-primary outline-none transition-all font-mono"
                    />
                  </div>

                  {provider.id !== 'ollama' && (
                    <div className="space-y-2">
                      <label className="text-xs font-semibold text-text-muted uppercase tracking-wider flex items-center gap-2">
                        <Key size={12} /> API Key
                      </label>
                      <input 
                        type="password"
                        placeholder="sk-..."
                        value={provider.api_key || ''}
                        onChange={(e) => handleKeyChange(provider.id, e.target.value)}
                        className="w-full bg-[#1e1e1e] border border-border rounded px-3 py-2 text-sm text-text focus:border-primary outline-none transition-all font-mono"
                      />
                    </div>
                  )}
                </div>
                <div className="p-3 bg-background/50 rounded border border-border/50 text-xs text-text-muted italic">
                  Agent requests will be routed to {provider.name} ({provider.model}) via {provider.endpoint}.
                </div>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="mt-12 pt-8 border-t border-border">
        <h3 className="text-sm font-semibold text-text mb-4 uppercase tracking-widest text-text-muted">Security Policy</h3>
        <p className="text-xs text-text-muted leading-relaxed">
          Your API credentials are saved locally in your application configuration folder. Sandman never transmits your keys to any central server; they are only sent directly to the configured provider endpoints during agent execution.
        </p>
      </div>
    </div>
  );
}
