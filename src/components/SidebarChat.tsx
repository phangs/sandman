import { useState, useRef, useEffect } from 'react';
import { Send, Bot, User, CornerDownRight, Plus, Check, MessageSquare, History, PlusSquare, Trash2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface Message {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

interface ChatSession {
  id: number;
  title: string;
  updated_at: number;
}

export function SidebarChat() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [showHistory, setShowHistory] = useState(false);

  const [messages, setMessages] = useState<Message[]>([
    { role: 'assistant', content: 'Brainstorming session active. How can I help with your project?' }
  ]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const [isCreating, setIsCreating] = useState(false);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    loadSessions();
  }, []);

  useEffect(() => {
    if (activeSessionId) {
        loadMessages(activeSessionId);
    } else {
        setMessages([{ role: 'assistant', content: 'Select or create a new session to begin brainstorming.' }]);
    }
  }, [activeSessionId]);

  const loadSessions = async () => {
    try {
      const res = await invoke<ChatSession[]>('list_chat_sessions');
      setSessions(res);
      if (res.length > 0 && !activeSessionId) {
        setActiveSessionId(res[0].id);
      }
    } catch (err) {
      console.error("Load sessions error:", err);
    }
  };

  const loadMessages = async (sid: number) => {
    try {
      const res = await invoke<any[]>('get_chat_messages', { sessionId: sid });
      if (res.length === 0) {
        setMessages([{ role: 'assistant', content: 'New session started. What are we brainstorming today?' }]);
      } else {
        setMessages(res.map(m => ({ role: m.role, content: m.content })));
      }
    } catch (err) {
      console.error("Load messages error:", err);
    }
  };

  const createNewSession = async () => {
    try {
      const title = `Session ${new Date().toLocaleTimeString()}`;
      const sid = await invoke<number>('create_chat_session', { title });
      await loadSessions();
      setActiveSessionId(sid);
      setShowHistory(false);
    } catch (err) {
      console.error("Create session error:", err);
    }
  };

  const deleteSession = async (id: number) => {
    try {
      await invoke('delete_chat_session', { id });
      if (activeSessionId === id) setActiveSessionId(null);
      await loadSessions();
    } catch (err) {
      console.error("Delete session error:", err);
    }
  };

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const createRequirement = async () => {
    if (messages.length <= 1 || isCreating) return;
    setIsCreating(true);
    try {
      await invoke('create_story_from_chat', { messages });
      setSuccess(true);
      setTimeout(() => setSuccess(false), 2000);
    } catch (err) {
      console.error("Failed to create requirement:", err);
    } finally {
      setIsCreating(false);
    }
  };

  const sendMessage = async () => {
    if (!input.trim() || isLoading) return;

    const userMsg: Message = { role: 'user', content: input };
    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setInput('');
    setIsLoading(true);

    try {
      const response = await invoke<string>('chat_with_agent', { 
        sessionId: activeSessionId,
        messagesAll: newMessages.map(m => ({
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
      <div className="px-3 py-2 flex items-center justify-between border-b border-border text-[10px] uppercase font-bold text-text-muted tracking-widest relative">
        <div className="flex gap-2 items-center">
            <button 
                onClick={() => setShowHistory(!showHistory)}
                className={`p-1 rounded-md transition-all ${showHistory ? 'bg-primary text-white' : 'hover:bg-surface text-text-muted hover:text-primary'}`}
                title="Session History"
            >
                <History size={12} />
            </button>
            <span>Brainstorm</span>
        </div>
        
        <div className="flex gap-1.5 items-center">
            <button 
                onClick={createNewSession}
                className="p-1 rounded-md hover:bg-surface text-text-muted hover:text-primary transition-all"
                title="New Chat Session"
            >
                <PlusSquare size={12} />
            </button>

            {messages.length > 1 && (
                <button 
                  onClick={createRequirement}
                  disabled={isCreating}
                  title="Create Requirement"
                  className={`p-1 rounded-md transition-all ${
                    success ? 'bg-green-500/20 text-green-500' : 'hover:bg-primary/20 text-text-muted hover:text-primary'
                  }`}
                >
                  {success ? <Check size={12} /> : <Plus size={12} />}
                </button>
            )}
            {isLoading && <span className="w-1.5 h-1.5 bg-primary rounded-full animate-ping" />}
        </div>

        {showHistory && (
            <div className="absolute top-full left-0 right-0 bg-[#1e1e1e] border-b border-border z-20 max-h-48 overflow-y-auto shadow-xl">
                {sessions.map(s => (
                    <div 
                        key={s.id}
                        className={`flex items-center justify-between px-3 py-2 cursor-pointer border-l-2 transition-all ${
                            activeSessionId === s.id ? 'bg-surface border-primary' : 'hover:bg-surface border-transparent'
                        }`}
                        onClick={() => {
                            setActiveSessionId(s.id);
                            setShowHistory(false);
                        }}
                    >
                        <div className="flex items-center gap-2 overflow-hidden">
                            <MessageSquare size={10} className={activeSessionId === s.id ? 'text-primary' : 'text-text-muted'} />
                            <span className={`truncate text-[9px] lowercase ${activeSessionId === s.id ? 'text-text font-bold' : 'text-text-muted'}`}>
                                {s.title}
                            </span>
                        </div>
                        <button 
                            onClick={(e) => {
                                e.stopPropagation();
                                deleteSession(s.id);
                            }}
                            className="p-1 hover:text-red-400 transition-colors"
                        >
                            <Trash2 size={10} />
                        </button>
                    </div>
                ))}
                {sessions.length === 0 && (
                    <div className="px-3 py-4 text-center text-text-muted italic lowercase opacity-50">
                        no sessions found
                    </div>
                )}
            </div>
        )}
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

        {sessions.length > 0 && (
            <div className="mt-3 flex gap-2 overflow-x-auto pb-1 no-scrollbar border-t border-border/20 pt-2">
                {sessions.slice(0, 3).map(s => (
                    <button 
                        key={s.id}
                        onClick={() => setActiveSessionId(s.id)}
                        className={`px-2 py-1 rounded border flex items-center gap-1.5 transition-all text-[9.5px] whitespace-nowrap lowercase group ${
                            activeSessionId === s.id 
                                ? 'bg-primary/20 border-primary text-primary font-bold active-glow shadow-[0_0_8px_-2px_var(--tw-shadow-color)] shadow-primary/30' 
                                : 'bg-surface border-border hover:border-primary/50 text-text-muted hover:text-primary hover:bg-primary/5'
                        }`}
                    >
                        <MessageSquare size={10} className={activeSessionId === s.id ? 'opacity-100' : 'opacity-40 group-hover:opacity-100 transition-opacity'} />
                        <span className="truncate max-w-[50px]">{s.title}</span>
                    </button>
                ))}
            </div>
        )}
      </div>
    </div>
  );
}
