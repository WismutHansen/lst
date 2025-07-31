import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useTheme } from "../hooks/useTheme";

export function ThemeSelector() {
  const { themeData, availableThemes, loading, applyTheme } = useTheme();
  const [isOpen, setIsOpen] = useState(false);

  if (loading) {
    return (
      <Button variant="outline" size="sm" disabled>
        Loading...
      </Button>
    );
  }

  return (
    <div className="relative">
      <Button
        variant="outline"
        size="sm"
        onClick={() => setIsOpen(!isOpen)}
        className="min-w-[120px] justify-between"
      >
        {themeData?.name || "Default"}
        <span className="ml-2">▼</span>
      </Button>
      
      {isOpen && (
        <div className="absolute top-full left-0 mt-1 w-full min-w-[200px] bg-popover border border-border rounded-md shadow-lg z-50">
          <div className="p-1">
            {availableThemes.map((themeName) => (
              <button
                key={themeName}
                onClick={() => {
                  applyTheme(themeName);
                  setIsOpen(false);
                }}
                className={`w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground ${
                  themeData?.name === themeName ? "bg-accent text-accent-foreground" : ""
                }`}
              >
                {themeName}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}