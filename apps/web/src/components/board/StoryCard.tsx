import {
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import type { Story, StoryState } from "@/data/mock_stories";
import { AlertCircle, CheckCircle2, Loader2, PlayCircle, RefreshCcw, GripVertical } from "lucide-react";
import { cn } from "@/lib/utils";

interface StoryCardProps {
  story: Story;
  onToggleAI?: (id: string, enabled: boolean) => void;
  onRetry?: (id: string) => void;
  onClick?: (story: Story) => void;
}

const getStateConfig = (state: StoryState) => {
  switch (state) {
    case 'Draft':
      return {
        borderColor: 'border-gray-500',
        icon: null,
        label: 'Draft',
        bg: 'bg-gray-100 dark:bg-gray-800'
      };
    case 'Ready':
      return {
        borderColor: 'border-blue-500 shadow-[0_0_10px_rgba(59,130,246,0.5)]',
        icon: <PlayCircle className="w-4 h-4 text-blue-500" />,
        label: 'Ready',
        bg: 'bg-blue-50 dark:bg-blue-950'
      };
    case 'Processing':
      return {
        borderColor: 'border-yellow-500 animate-pulse',
        icon: <Loader2 className="w-4 h-4 text-yellow-500 animate-spin" />,
        label: 'AI Processing',
        bg: 'bg-yellow-50 dark:bg-yellow-950'
      };
    case 'Failed':
      return {
        borderColor: 'border-red-500',
        icon: <AlertCircle className="w-4 h-4 text-red-500" />,
        label: 'Failed',
        bg: 'bg-red-50 dark:bg-red-950'
      };
    case 'Success':
      return {
        borderColor: 'border-green-500',
        icon: <CheckCircle2 className="w-4 h-4 text-green-500" />,
        label: 'Success',
        bg: 'bg-green-50 dark:bg-green-950'
      };
  }
};

export function StoryCard({ story, onToggleAI, onRetry, onClick }: StoryCardProps) {
  const config = getStateConfig(story.state);
  
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: story.id,
    data: {
      type: "Story",
      story,
    },
  });

  const style = {
    transition,
    transform: CSS.Translate.toString(transform),
  };

  if (isDragging) {
    return (
      <div
        ref={setNodeRef}
        style={style}
        className={cn(
          "opacity-30 border-2 border-dashed h-[120px] rounded-lg mb-3",
          config.borderColor
        )}
      />
    );
  }

  return (
    <Card 
      ref={setNodeRef}
      style={style}
      className={cn(
        "cursor-pointer transition-all hover:shadow-md mb-3 group relative",
        config.borderColor
      )}
      onClick={() => onClick?.(story)}
    >
      <div 
        {...attributes} 
        {...listeners}
        className="absolute right-1 top-1 p-1 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab active:cursor-grabbing"
      >
        <GripVertical className="w-3 h-3 text-muted-foreground" />
      </div>
      
      <CardHeader className="p-3 pb-1">
        <div className="flex justify-between items-start">
          <span className="text-xs font-mono text-muted-foreground">{story.id}</span>
          <Badge variant="outline" className="text-[10px] px-1 h-4">
            {config.label}
          </Badge>
        </div>
        <CardTitle className="text-sm font-bold leading-tight mt-1">
          {story.title}
        </CardTitle>
      </CardHeader>
      <CardContent className="p-3 pt-1">
        <p className="text-xs text-muted-foreground line-clamp-2">
          {story.description}
        </p>
      </CardContent>
      <CardFooter className="p-3 pt-0 flex justify-between items-center mt-2">
        <div className="flex items-center gap-2">
          {config.icon}
          {story.state === 'Failed' && (
             <Button 
                variant="outline" 
                size="sm" 
                className="h-7 text-[10px] uppercase font-bold border-red-500 text-red-500 hover:bg-red-500 hover:text-white transition-colors"
                onClick={(e) => {
                  e.stopPropagation();
                  onRetry?.(story.id);
                }}
             >
               <RefreshCcw className="w-3 h-3 mr-1" />
               Retry
             </Button>
          )}
        </div>
        <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
          <span className="text-[10px] text-muted-foreground">Ready for AI</span>
          <Switch 
            checked={story.isReadyForAI} 
            onCheckedChange={(checked) => onToggleAI?.(story.id, checked)}
            className="scale-75"
          />
        </div>
      </CardFooter>
    </Card>
  );
}
