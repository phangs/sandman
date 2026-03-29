import { useState, useEffect, useCallback } from "react";
import {
  DndContext,
  DragOverlay,
  closestCorners,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  defaultDropAnimationSideEffects,
} from "@dnd-kit/core";
import type { 
  DragStartEvent,
  DragOverEvent,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  sortableKeyboardCoordinates,
} from "@dnd-kit/sortable";
import type { Story, StoryState } from "@/data/mock_stories";
import { Column } from "./Column";
import { StoryCard } from "./StoryCard";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { RefreshCcw, Loader2 } from "lucide-react";
import { fetchStories, updateStory, SOCKET_URL } from "@/lib/api";
import { io } from "socket.io-client";

const COLUMNS: { id: StoryState; title: string }[] = [
  { id: 'Draft', title: 'Backlog' },
  { id: 'Ready', title: 'To Do' },
  { id: 'Processing', title: 'In Progress' },
  { id: 'Failed', title: 'Review / Failed' },
  { id: 'Success', title: 'Done' },
];

export function KanbanBoard() {
  const [stories, setStories] = useState<Story[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeStory, setActiveStory] = useState<Story | null>(null);
  const [selectedStory, setSelectedStory] = useState<Story | null>(null);

  useEffect(() => {
    const socket = io(SOCKET_URL);

    const loadStories = async () => {
      try {
        const data = await fetchStories();
        setStories(data);
      } catch (err) {
        console.error("Failed to load stories:", err);
      } finally {
        setLoading(false);
      }
    };

    loadStories();

    socket.on('storyUpdated', (updatedStory: Story) => {
      setStories(prev => prev.map(s => s.id === updatedStory.id ? updatedStory : s));
      setSelectedStory(prev => prev?.id === updatedStory.id ? updatedStory : prev);
    });

    socket.on('logAdded', ({ storyId, log }: { storyId: string, log: string }) => {
      setStories(prev => prev.map(s => {
        if (s.id === storyId) {
          return { ...s, logs: [...(s.logs || []), log] };
        }
        return s;
      }));
      setSelectedStory(prev => {
        if (prev?.id === storyId) {
          return { ...prev, logs: [...(prev.logs || []), log] };
        }
        return prev;
      });
    });

    return () => {
      socket.disconnect();
    };
  }, []);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 5,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleToggleAI = async (id: string, enabled: boolean) => {
    try {
      await updateStory(id, { isReadyForAI: enabled });
    } catch (err) {
      console.error("Failed to toggle AI status:", err);
    }
  };

  const handleRetry = async (id: string) => {
    try {
      await updateStory(id, { state: 'Processing' });
    } catch (err) {
      console.error("Failed to retry story:", err);
    }
  };

  const getStoriesByState = useCallback((state: StoryState) => {
    return stories.filter(s => s.state === state);
  }, [stories]);

  const onDragStart = (event: DragStartEvent) => {
    if (event.active.data.current?.type === "Story") {
      setActiveStory(event.active.data.current.story);
    }
  };

  const onDragOver = async (event: DragOverEvent) => {
    const { active, over } = event;
    if (!over) return;

    const activeId = active.id as string;
    const overId = over.id as string;

    if (activeId === overId) return;

    const isActiveAStory = active.data.current?.type === "Story";
    const isOverAStory = over.data.current?.type === "Story";
    const isOverAColumn = over.data.current?.type === "Column";

    if (!isActiveAStory) return;

    // Dropping a story over another story
    if (isActiveAStory && isOverAStory) {
      const activeIndex = stories.findIndex((s) => s.id === activeId);
      const overIndex = stories.findIndex((s) => s.id === overId);

      if (stories[activeIndex].state !== stories[overIndex].state) {
        const newState = stories[overIndex].state;
        setStories((prev) => {
          const newStories = [...prev];
          newStories[activeIndex] = { ...newStories[activeIndex], state: newState };
          return arrayMove(newStories, activeIndex, overIndex);
        });
        
        try {
          await updateStory(activeId, { state: newState });
        } catch (err) {
          console.error("Failed to update story state on drag:", err);
        }
      } else {
        setStories((prev) => arrayMove(prev, activeIndex, overIndex));
      }
    }

    // Dropping a story over a column
    if (isActiveAStory && isOverAColumn) {
      const activeIndex = stories.findIndex((s) => s.id === activeId);
      if (stories[activeIndex].state === overId) return;
      
      const newState = overId as StoryState;
      setStories((prev) => {
        const newStories = [...prev];
        newStories[activeIndex] = { ...newStories[activeIndex], state: newState };
        return arrayMove(newStories, activeIndex, activeIndex);
      });

      try {
        await updateStory(activeId, { state: newState });
      } catch (err) {
        console.error("Failed to update story state on drag over column:", err);
      }
    }
  };

  const onDragEnd = (_event: DragEndEvent) => {
    setActiveStory(null);
  };

  if (loading) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-slate-100 dark:bg-slate-950">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDragEnd={onDragEnd}
    >
      <div className="flex gap-6 h-full overflow-x-auto p-6 pb-12 items-start bg-slate-100 dark:bg-slate-950">
        {COLUMNS.map((col) => (
          <Column 
            key={col.id} 
            id={col.id} 
            title={col.title} 
            stories={getStoriesByState(col.id)}
            onToggleAI={handleToggleAI}
            onRetry={handleRetry}
            onStoryClick={setSelectedStory}
            />
        ))}
      </div>

      <DragOverlay dropAnimation={{
        sideEffects: defaultDropAnimationSideEffects({
          styles: {
            active: {
              opacity: '0.5',
            },
          },
        }),
      }}>
        {activeStory ? (
          <StoryCard 
            story={activeStory} 
          />
        ) : null}
      </DragOverlay>

      <Dialog open={!!selectedStory} onOpenChange={() => setSelectedStory(null)}>
        <DialogContent className="max-w-2xl bg-white dark:bg-slate-900">
          <DialogHeader>
            <div className="flex items-center gap-2 mb-2">
              <span className="text-xs font-mono text-muted-foreground">{selectedStory?.id}</span>
              <span className="px-2 py-0.5 rounded text-[10px] font-bold uppercase bg-slate-100 dark:bg-slate-800">
                {selectedStory?.state}
              </span>
            </div>
            <DialogTitle className="text-xl">{selectedStory?.title}</DialogTitle>
            {selectedStory?.state === 'Failed' && (
              <div className="mt-2">
                <Button 
                  variant="destructive" 
                  size="sm" 
                  className="h-8 uppercase font-bold text-[10px]"
                  onClick={() => {
                    handleRetry(selectedStory.id);
                    setSelectedStory(prev => prev ? { ...prev, state: 'Processing' } : null);
                  }}
                >
                  <RefreshCcw className="w-3 h-3 mr-2" />
                  Retry Build
                </Button>
              </div>
            )}
          </DialogHeader>
          
          <div className="space-y-6 py-4">
            <div>
              <h4 className="text-sm font-semibold mb-2">Requirement</h4>
              <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
                {selectedStory?.description}
              </p>
            </div>

            {selectedStory?.acceptanceCriteria && (
              <div>
                <h4 className="text-sm font-semibold mb-2">Acceptance Criteria</h4>
                <ul className="list-disc list-inside space-y-1">
                  {selectedStory.acceptanceCriteria.map((ac, i) => (
                    <li key={i} className="text-sm text-slate-600 dark:text-slate-400">{ac}</li>
                  ))}
                </ul>
              </div>
            )}

            {selectedStory?.logs && (
              <div>
                <h4 className="text-sm font-semibold mb-2">Recent Logs</h4>
                <div className="bg-slate-950 p-3 rounded-md font-mono text-xs space-y-1 text-slate-300">
                  {selectedStory.logs.map((log, i) => (
                    <p key={i}>{log}</p>
                  ))}
                </div>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </DndContext>
  );
}
