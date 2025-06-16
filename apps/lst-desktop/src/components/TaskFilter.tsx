
import { TaskFilter as FilterType } from "@/types/Task";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface TaskFilterProps {
  activeFilter: FilterType;
  onFilterChange: (filter: FilterType) => void;
  taskCounts: {
    all: number;
    active: number;
    completed: number;
  };
}

const TaskFilter = ({ activeFilter, onFilterChange, taskCounts }: TaskFilterProps) => {
  const filters: { value: FilterType; label: string }[] = [
    { value: "all", label: `All (${taskCounts.all})` },
    { value: "active", label: `Active (${taskCounts.active})` },
    { value: "completed", label: `Completed (${taskCounts.completed})` },
  ];

  return (
    <div className="flex flex-wrap gap-2 mb-6">
      {filters.map((filter) => (
        <Button
          key={filter.value}
          variant={activeFilter === filter.value ? "default" : "outline"}
          size="sm"
          onClick={() => onFilterChange(filter.value)}
          className={cn(
            "transition-all",
            activeFilter === filter.value && "shadow-md"
          )}
        >
          {filter.label}
        </Button>
      ))}
    </div>
  );
};

export default TaskFilter;
