
import { Task } from "@/types/Task";
import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import { Trash } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Card,
  CardContent,
} from "@/components/ui/card";

interface TaskItemProps {
  task: Task;
  onToggleComplete: (id: string) => void;
  onDelete: (id: string) => void;
}

const TaskItem = ({ task, onToggleComplete, onDelete }: TaskItemProps) => {
  return (
    <Card className={cn(
      "task-item mb-3 border shadow-sm hover:shadow-md transition-shadow",
      task.completed && "completed bg-secondary/50"
    )}>
      <CardContent className="p-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Checkbox
            checked={task.completed}
            onCheckedChange={() => onToggleComplete(task.id)}
            className="h-5 w-5"
          />
          <div>
            <h3 className={cn(
              "task-text text-base",
              task.completed && "text-muted-foreground line-through"
            )}>
              {task.title}
            </h3>
            {task.description && (
              <p className={cn(
                "text-sm text-muted-foreground mt-1",
                task.completed && "line-through"
              )}>
                {task.description}
              </p>
            )}
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => onDelete(task.id)}
          className="text-muted-foreground hover:text-destructive"
        >
          <Trash size={16} />
          <span className="sr-only">Delete task</span>
        </Button>
      </CardContent>
    </Card>
  );
};

export default TaskItem;
