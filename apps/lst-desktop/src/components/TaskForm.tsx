
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import { 
  Card,
  CardContent,
} from "@/components/ui/card";
import { useToast } from "@/components/ui/use-toast";

interface TaskFormProps {
  onAddTask: (title: string, description: string) => void;
}

const TaskForm = ({ onAddTask }: TaskFormProps) => {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const { toast } = useToast();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!title.trim()) {
      toast({
        title: "Task title required",
        description: "Please enter a task title",
        variant: "destructive",
      });
      return;
    }

    onAddTask(title.trim(), description.trim());
    setTitle("");
    setDescription("");
    
    toast({
      title: "Task added",
      description: "Your task has been added successfully",
    });
  };

  return (
    <Card className="mb-6 border shadow-sm">
      <CardContent className="p-4">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <Input
              placeholder="What needs to be done?"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="text-base"
              autoFocus
            />
          </div>
          <div>
            <Input
              placeholder="Add description (optional)"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="text-sm"
            />
          </div>
          <div className="flex justify-end">
            <Button type="submit" className="px-4 py-2">
              <Plus size={16} className="mr-2" />
              Add Task
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
};

export default TaskForm;
