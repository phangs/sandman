import { useDroppable } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import type { Story } from "@/data/mock_stories";
import { StoryCard } from "./StoryCard";

interface ColumnProps {
  id: string;
  title: string;
  stories: Story[];
  onToggleAI?: (id: string, enabled: boolean) => void;
  onRetry?: (id: string) => void;
  onStoryClick?: (story: Story) => void;
}

export function Column({ id, title, stories, onToggleAI, onRetry, onStoryClick }: ColumnProps) {
  const { setNodeRef } = useDroppable({
    id,
    data: {
      type: "Column",
      id,
    },
  });

  return (
    <div 
      ref={setNodeRef}
      className="flex flex-col h-full min-w-[300px] max-w-[350px] bg-slate-50 dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 shadow-sm"
    >
      <div className="p-4 border-b border-slate-200 dark:border-slate-800 bg-slate-100/50 dark:bg-slate-900/50 rounded-t-lg">
        <div className="flex items-center justify-between">
          <h3 className="font-semibold text-sm uppercase tracking-wider text-slate-500">
            {title}
          </h3>
          <span className="text-xs bg-slate-200 dark:bg-slate-800 text-slate-600 dark:text-slate-400 px-2 py-0.5 rounded-full">
            {stories.length}
          </span>
        </div>
      </div>
      <div className="p-3 flex-1 overflow-y-auto space-y-3 min-h-[150px]">
        <SortableContext items={stories.map(s => s.id)} strategy={verticalListSortingStrategy}>
          {stories.map((story) => (
            <StoryCard 
              key={story.id} 
              story={story} 
              onToggleAI={onToggleAI} 
              onRetry={onRetry}
              onClick={onStoryClick}
            />
          ))}
        </SortableContext>
        {stories.length === 0 && (
          <div className="h-full flex items-center justify-center border-2 border-dashed border-slate-200 dark:border-slate-800 rounded-lg min-h-[100px]">
            <span className="text-xs text-slate-400">Drop here</span>
          </div>
        )}
      </div>
    </div>
  );
}
