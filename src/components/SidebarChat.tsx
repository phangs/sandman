import { useState, useRef, useEffect } from 'react';
import { Send, Bot, User, CornerDownRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface Message {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export function SidebarChat() {
  const [messages, setMessages] = useState<Message[]>([
    { role: 'assistant', content: 'Brainstorming session active. How can I help with your project?' }
  ]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const sendMessage = async () => {
    if (!input.trim() || isLoading) return;

    const userMsg: Message = { role: 'user', content: input };
    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setInput('');
    setIsLoading(true);

    try {
      // Map to Rust-compatible message format
      const response = await invoke<string>('chat_with_agent', { 
        messages: newMessages.map(m => ({
          role: m.role,
          content: m.content
        }))
      });
      
      setMessages([...newMessages, { role: 'assistant', content: response }]);
    } catch (err) {
      console.error("Chat error:", err);
      setMessages([...newMessages, { role: 'assistant', content: `Error: ${err}` }]);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-1/2 border-t border-border bg-[#252526]">
      <div className="px-3 py-2 flex items-center justify-between border-b border-border text-[10px] uppercase font-bold text-text-muted tracking-widest">
        <span>Brainstorm</span>
        <div className="flex gap-1.5 items-center">
            {isLoading && <span className="w-1.5 h-1.5 bg-primary rounded-full animate-ping" />}
            <span className={isLoading ? 'text-primary' : 'text-text-muted'}>
                {isLoading ? 'Agent Thinking...' : 'Ready'}
            </span>
        </div>
      </div>

      <div ref={scrollRef} className="flex-1 p-3 overflow-y-auto space-y-4">
        {messages.map((m, i) => (
          <div key={i} className={`flex gap-2 ${m.role === 'user' ? 'flex-row-reverse' : 'flex-row'}`}>
            <div className={`mt-0.5 p-1 rounded-full h-fit w-fit ${m.role === 'user' ? 'bg-primary/20 text-primary' : 'bg-surface text-text-muted'}`}>
              {m.role === 'user' ? <User size={12} /> : <Bot size={12} />}
            </div>
            <div className={`text-xs p-2 rounded-lg max-w-[85%] leading-relaxed ${
              m.role === 'user' ? 'bg-primary text-white ml-2 rounded-tr-none' : 'bg-surface text-text-muted mr-2 rounded-tl-none border border-border/30'
            }`}>
              {m.content}
            </div>
          </div>
        ))}
      </div>

      <div className="p-3 border-t border-border bg-background">
        <div className="relative group">
          <textarea
            rows={1}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendMessage();
              }
            }}
            placeholder="Ask anything..."
            className="w-full bg-[#1e1e1e] text-xs text-text border border-border rounded-md pl-3 pr-8 py-2 outline-none focus:border-primary transition-all resize-none max-h-32"
          />
          <button 
            onClick={sendMessage}
            disabled={isLoading || !input.trim()}
            className="absolute right-2 bottom-2 text-text-muted hover:text-primary disabled:opacity-30 disabled:hover:text-text-muted transition-colors"
          >
            <Send size={14} />
          </button>
        </div>
        <div className="mt-1 flex items-center gap-1 text-[9px] text-text-muted italic">
            <CornerDownRight size={10} /> Shift+Enter for new line
        </div>
      </div>
    </div>
  );
}
