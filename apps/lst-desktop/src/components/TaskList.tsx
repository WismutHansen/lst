
import { Task, TaskFilter } from "@/types/Task";
import TaskItem from "./TaskItem";
import { Card, CardContent } from "@/components/ui/card";

interface TaskListProps {
  tasks: Task[];
  activeFilter: TaskFilter;
  onToggleComplete: (id: string) => void;
  onDelete: (id: string) => void;
}

const TaskList = ({ tasks, activeFilter, onToggleComplete, onDelete }: TaskListProps) => {
  const filteredTasks = tasks.filter((task) => {
    if (activeFilter === "all") return true;
    if (activeFilter === "active") return !task.completed;
    if (activeFilter === "completed") return task.completed;
    return true;
  });

  if (filteredTasks.length === 0) {
    return (
      <Card className="border shadow-sm">
        <CardContent className="p-6 text-center">
          <p className="text-muted-foreground">
            {activeFilter === "all"
              ? "No tasks yet. Add a task to get started!"
              : activeFilter === "active"
              ? "No active tasks. All done!"
              : "No completed tasks yet."}
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-3">
      {filteredTasks.map((task) => (
        <TaskItem
          key={task.id}
          task={task}
          onToggleComplete={onToggleComplete}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
};

export default TaskList;
