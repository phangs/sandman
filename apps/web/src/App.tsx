import { useState, useEffect } from 'react'
import { KanbanBoard } from '@/components/board/KanbanBoard'
import { Settings, Wifi, WifiOff, Terminal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { io } from 'socket.io-client'
import { SOCKET_URL } from '@/lib/api'

function App() {
  const [isConnected, setIsConnected] = useState(false)
  const [showTerminal, setShowTerminal] = useState(false)
  const [systemLogs, setSystemLogs] = useState<{ id: number, text: string, type: string, time: string }[]>([
    { id: 1, text: 'System initialized.', type: 'info', time: new Date().toLocaleTimeString() }
  ])

  useEffect(() => {
    const socket = io(SOCKET_URL)

    socket.on('connect', () => {
      setIsConnected(true)
      addLog('Connected to Backend Orchestrator.', 'success')
    })

    socket.on('disconnect', () => {
      setIsConnected(false)
      addLog('Disconnected from Backend Orchestrator.', 'error')
    })

    socket.on('logAdded', ({ storyId, log }: { storyId: string, log: string }) => {
      addLog(`[${storyId}] ${log}`, 'info')
    })

    socket.on('storyUpdated', (story: any) => {
      addLog(`Story ${story.id} updated to state: ${story.state}`, 'info')
    })

    function addLog(text: string, type: string) {
      setSystemLogs(prev => [
        ...prev,
        { id: Date.now(), text, type, time: new Date().toLocaleTimeString() }
      ])
    }

    return () => {
      socket.disconnect()
    }
  }, [])

  const getLogColor = (type: string) => {
    switch (type) {
      case 'success': return 'text-green-500';
      case 'error': return 'text-red-500';
      case 'info': return 'text-blue-400';
      default: return 'text-zinc-300';
    }
  }

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-background text-foreground">
      {/* Header */}
      <header className="h-16 border-b border-border flex items-center justify-between px-6 bg-card shrink-0">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold tracking-tight">Sandman Command Center</h1>
          <div className="flex items-center gap-2">
            <Badge variant={isConnected ? "default" : "destructive"} className="flex gap-1 items-center px-2 py-0.5">
              {isConnected ? <Wifi className="w-3 h-3" /> : <WifiOff className="w-3 h-3" />}
              <span className="text-[10px] uppercase font-bold">{isConnected ? "Connected" : "Disconnected"}</span>
            </Badge>
            <Badge variant="outline" className="text-[10px] uppercase">v1.0.0-alpha</Badge>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Button variant="ghost" size="icon" onClick={() => setShowTerminal(!showTerminal)}>
            <Terminal className="w-5 h-5" />
          </Button>
          <Button variant="ghost" size="icon">
            <Settings className="w-5 h-5" />
          </Button>
        </div>
      </header>

      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Kanban Board */}
        <main className="flex-1 overflow-x-auto relative">
          <KanbanBoard />
        </main>

        {/* Side Panel (Terminal) */}
        <aside 
          className={`
            fixed right-0 top-16 bottom-0 w-96 bg-zinc-950 border-l border-zinc-800 transition-transform duration-300 z-50
            ${showTerminal ? 'translate-x-0' : 'translate-x-full'}
          `}
        >
          <div className="flex flex-col h-full">
            <div className="p-4 border-b border-zinc-800 flex justify-between items-center">
              <h2 className="text-sm font-mono text-zinc-400">System Logs</h2>
              <Button variant="ghost" size="sm" className="h-6 text-zinc-400 hover:text-white" onClick={() => setShowTerminal(false)}>
                Close
              </Button>
            </div>
            <div className="flex-1 p-4 font-mono text-xs overflow-y-auto space-y-2 text-zinc-300">
              {systemLogs.map(log => (
                <p key={log.id}>
                  <span className="text-zinc-500 mr-2">{log.time} -</span>
                  <span className={getLogColor(log.type)}>{log.text}</span>
                </p>
              ))}
              <div className="pt-4 opacity-50 italic">Waiting for activity...</div>
            </div>
          </div>
        </aside>
      </div>
    </div>
  )
}

export default App
