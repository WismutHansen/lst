import { Command } from "cmdk";
import { useEffect } from "react";

export interface PaletteCommand {
  label: string;
  action: () => void;
}

interface Props {
  open: boolean;
  onClose: () => void;
  commands: PaletteCommand[];
}

export function CommandPalette({ open, onClose, commands }: Props) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    if (open) document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/40"
      onClick={onClose}
    >
      <Command
        className="bg-neutral-800 text-white rounded-md w-80 max-h-60 overflow-y-auto shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        {commands.map((cmd) => (
          <Command.Item
            key={cmd.label}
            onSelect={() => {
              cmd.action();
              onClose();
            }}
            className="px-3 py-2 cursor-pointer aria-selected:bg-neutral-700"
          >
            {cmd.label}
          </Command.Item>
        ))}
      </Command>
    </div>
  );
}
