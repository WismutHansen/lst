import { useState, useMemo, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useTheme } from "../hooks/useTheme";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Search } from "lucide-react";

// Simple fuzzy search function
function fuzzySearch(query: string, text: string): boolean {
  if (!query) return true;
  
  const queryLower = query.toLowerCase();
  const textLower = text.toLowerCase();
  
  // Simple contains check first
  if (textLower.includes(queryLower)) return true;
  
  // Fuzzy match: check if all query characters appear in order
  let queryIndex = 0;
  for (let i = 0; i < textLower.length && queryIndex < queryLower.length; i++) {
    if (textLower[i] === queryLower[queryIndex]) {
      queryIndex++;
    }
  }
  
  return queryIndex === queryLower.length;
}

export function MobileThemeSelector() {
  const { themeData, availableThemes, loading, applyTheme } = useTheme();
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Filter themes based on search query
  const filteredThemes = useMemo(() => {
    return availableThemes.filter(themeName => 
      fuzzySearch(searchQuery, themeName)
    );
  }, [availableThemes, searchQuery]);

  // Reset selected index when search changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [searchQuery]);

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex(prev => Math.min(prev + 1, filteredThemes.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex(prev => Math.max(prev - 1, 0));
    } else if (e.key === "Enter" && filteredThemes[selectedIndex]) {
      e.preventDefault();
      applyTheme(filteredThemes[selectedIndex]);
      setIsOpen(false);
      setSearchQuery("");
    }
  };

  if (loading) {
    return (
      <Button variant="outline" size="sm" disabled>
        Loading...
      </Button>
    );
  }

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="min-w-[100px]">
          {themeData?.name || "Default"}
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Select Theme</DialogTitle>
        </DialogHeader>
        
        {/* Search Bar */}
        <div className="relative mb-4">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search themes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            className="pl-10"
            autoFocus
          />
        </div>
        
        {/* Theme List */}
        <div className="flex-1 overflow-y-auto space-y-2 max-h-[400px]">
          {filteredThemes.length === 0 ? (
            <p className="text-center text-muted-foreground py-4">
              No themes found matching "{searchQuery}"
            </p>
          ) : (
            filteredThemes.map((themeName, index) => (
              <Button
                key={themeName}
                variant={themeData?.name === themeName ? "default" : "outline"}
                onClick={() => {
                  applyTheme(themeName);
                  setIsOpen(false);
                  setSearchQuery(""); // Reset search when closing
                }}
                className={`w-full justify-start ${
                  index === selectedIndex ? "ring-2 ring-primary" : ""
                }`}
              >
                {themeName}
              </Button>
            ))
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}