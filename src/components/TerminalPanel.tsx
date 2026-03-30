import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Terminal } from 'xterm';
import { FitAddon } from '@xterm/addon-fit';
import 'xterm/css/xterm.css';

interface LogLine {
  id: number;
  raw: string;
  parts: { text: string; color?: string; bold?: boolean }[];
}

function parseAnsi(raw: string): { text: string; color?: string; bold?: boolean }[] {
  const parts: { text: string; color?: string; bold?: boolean }[] = [];
  const regex = /\x1b\[([0-9;]*)m/g;
  let lastIndex = 0;
  let color: string | undefined;
  let bold = false;

  const colorMap: Record<string, string> = {
    '30': '#44475a', '31': '#ff5555', '32': '#50fa7b', '33': '#f1fa8c',
    '34': '#8be9fd', '35': '#a78bfa', '36': '#8be9fd', '37': '#e0e0e0',
    '39': '#e0e0e0',
    '90': '#6272a4', '91': '#ff5555', '92': '#50fa7b', '93': '#f1fa8c',
    '94': '#8be9fd', '95': '#ff79c6', '96': '#8be9fd', '97': '#ffffff',
  };

  let match: RegExpExecArray | null;
  while ((match = regex.exec(raw)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ text: raw.slice(lastIndex, match.index), color, bold });
    }
    const codes = match[1].split(';');
    for (const code of codes) {
      const c = code.trim();
      if (c === '0' || c === '') { color = undefined; bold = false; }
      else if (c === '1') bold = true;
      else if (colorMap[c]) color = colorMap[c];
    }
    lastIndex = regex.lastIndex;
  }
  if (lastIndex < raw.length) {
    parts.push({ text: raw.slice(lastIndex), color, bold });
  }
  return parts.filter(p => p.text.length > 0);
}

let lineIdCounter = 0;

export function TerminalPanel() {
  const [activeTab, setActiveTab] = useState<'output' | 'terminal'>('output');
  const [outputLines, setOutputLines] = useState<LogLine[]>([
    { id: lineIdCounter++, raw: '', parts: [{ text: '[Sandman] ', color: '#a78bfa', bold: true }, { text: 'Agentic Engine Initialized.' }] },
    { id: lineIdCounter++, raw: '', parts: [{ text: 'Listening for autonomous agent events...', color: '#6272a4' }] },
  ]);
  
  const outputBottomRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    // 1. Output Logs Listener
    const unlistenLogs = listen<string>('log', (event) => {
      const raw = event.payload;
      setOutputLines(prev => [...prev.slice(-1000), {
        id: lineIdCounter++,
        raw,
        parts: parseAnsi(raw),
      }]);
    });

    // 2. Agent Terminal Output Listener (One-shot commands)
    const unlistenAgentTerminal = listen<string>('terminal-stdout', (event) => {
        const raw = event.payload;
        setOutputLines(prev => [...prev.slice(-1000), {
            id: lineIdCounter++,
            raw,
            parts: parseAnsi(raw),
        }]);
    });

    // 3. Xterm Init
    if (!xtermRef.current && terminalRef.current) {
        const term = new Terminal({
            cursorBlink: true,
            theme: {
                background: '#1a1a2e',
                foreground: '#ffffff',
                cursor: '#bd93f9',
                selectionBackground: '#44475a',
                black: '#21222c',
                red: '#ff5555',
                green: '#50fa7b',
                yellow: '#f1fa8c',
                blue: '#bd93f9',
                magenta: '#ff79c6',
                cyan: '#8be9fd',
                white: '#f8f8f2',
            },
            fontSize: 12,
            fontFamily: 'JetBrains Mono, monospace',
            allowProposedApi: true,
        });
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        term.open(terminalRef.current);
        fitAddon.fit();

        term.onData(data => {
            invoke('pty_write', { data }).catch(console.error);
        });

        // Resize detection
        const resizeObserver = new ResizeObserver(() => {
            if (fitAddon) {
                fitAddon.fit();
                invoke('pty_resize', { 
                    cols: term.cols, 
                    rows: term.rows 
                }).catch(console.error);
            }
        });
        resizeObserver.observe(terminalRef.current);

        xtermRef.current = term;
        fitAddonRef.current = fitAddon;
    }

    // 4. PTY Output Listener
    const unlistenPty = listen<string>('pty-stdout', (event) => {
        if (xtermRef.current) {
            xtermRef.current.write(event.payload);
        }
    });

    return () => { 
        unlistenLogs.then(fn => fn()); 
        unlistenAgentTerminal.then(fn => fn());
        unlistenPty.then(fn => fn());
    };
  }, []);

  useEffect(() => {
    if (activeTab === 'output') {
      outputBottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    } else if (activeTab === 'terminal' && fitAddonRef.current) {
      setTimeout(() => {
        fitAddonRef.current?.fit();
        xtermRef.current?.focus();
      }, 100);
    }
  }, [outputLines, activeTab]);

  return (
    <div className="w-full h-full flex flex-col bg-[#1a1a2e] overflow-hidden">
      {/* Tabs Header */}
      <div className="flex-shrink-0 h-9 bg-black/20 border-b border-white/5 flex items-center px-4 gap-6">
        <button 
          onClick={() => setActiveTab('output')}
          className={`h-full px-2 text-[10px] font-bold uppercase tracking-widest transition-all border-b-2 ${activeTab === 'output' ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text'}`}
        >
          Output
        </button>
        <button 
          onClick={() => setActiveTab('terminal')}
          className={`h-full px-2 text-[10px] font-bold uppercase tracking-widest transition-all border-b-2 ${activeTab === 'terminal' ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text'}`}
        >
          Terminal
        </button>
      </div>

      <div className="flex-1 relative overflow-hidden">
        {/* Output Tab Content */}
        <div 
            className={`absolute inset-0 overflow-y-auto px-4 py-3 custom-scrollbar transition-opacity duration-200 ${activeTab === 'output' ? 'opacity-100 z-10' : 'opacity-0 z-0 pointer-events-none'}`}
        >
            {outputLines.map((line) => (
              <div key={line.id} className="whitespace-pre-wrap break-all min-h-[1.4em] mb-0.5 font-mono text-xs">
                {line.parts.map((part, i) => (
                  <span key={i} style={{ color: part.color ?? '#e0e0e0', fontWeight: part.bold ? 700 : 400 }}>
                    {part.text}
                  </span>
                ))}
              </div>
            ))}
            <div ref={outputBottomRef} />
        </div>

        {/* Terminal Tab Content (Real Xterm) */}
        <div 
            ref={terminalRef}
            className={`absolute inset-0 px-2 py-1 transition-opacity duration-200 ${activeTab === 'terminal' ? 'opacity-100 z-10' : 'opacity-0 z-0 pointer-events-none'}`}
        />
      </div>
    </div>
  );
}
