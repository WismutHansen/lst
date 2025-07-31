import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useTheme } from "../hooks/useTheme";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

export function MobileThemeSelector() {
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
    <Sheet open={isOpen} onOpenChange={setIsOpen}>
      <SheetTrigger asChild>
        <Button variant="outline" size="sm" className="min-w-[100px]">
          {themeData?.name || "Default"}
        </Button>
      </SheetTrigger>
      <SheetContent side="bottom" className="h-[300px]">
        <SheetHeader>
          <SheetTitle>Select Theme</SheetTitle>
        </SheetHeader>
        <div className="grid gap-2 mt-4">
          {availableThemes.map((themeName) => (
            <Button
              key={themeName}
              variant={themeData?.name === themeName ? "default" : "outline"}
              onClick={() => {
                applyTheme(themeName);
                setIsOpen(false);
              }}
              className="justify-start"
            >
              {themeName}
            </Button>
          ))}
        </div>
      </SheetContent>
    </Sheet>
  );
}