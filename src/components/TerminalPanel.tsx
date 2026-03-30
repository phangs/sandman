import { useEffect, useRef } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from '@xterm/addon-fit';
import { listen } from '@tauri-apps/api/event';
import 'xterm/css/xterm.css';

export function TerminalPanel() {
  const terminalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    const term = new Terminal({
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#cccccc',
        selectionBackground: 'rgba(0, 122, 204, 0.3)',
      },
      fontFamily: "'Consolas', 'Courier New', monospace",
      fontSize: 13,
      disableStdin: true,
      cursorBlink: true,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.current);
    
    // Delay fit slightly to ensure parent container has exact dimensions
    setTimeout(() => fitAddon.fit(), 100);

    term.writeln('\x1b[32m~/sandman $\x1b[0m Sandman IPC Engine Initialized.');
    term.writeln('\x1b[90mListening for AI agent events via Rust WebSockets...\x1b[0m');

    const unlisten = listen<string>('log', (event) => {
      term.writeln(event.payload);
    });

    const observer = new ResizeObserver(() => {
      fitAddon.fit();
    });
    observer.observe(terminalRef.current);

    return () => {
      unlisten.then((fn) => fn());
      observer.disconnect();
      term.dispose();
    };
  }, []);

  return <div ref={terminalRef} className="w-full h-full overflow-hidden" />;
}
