import { useState, useEffect } from "react";
import { Badge } from "@/components/ui/badge";
import { Wifi, WifiOff, Loader2, AlertTriangle } from "lucide-react";
import { commands } from "../bindings";

interface SyncStatus {
  connected: boolean;
  lastSync: string | null;
  pendingChanges: number;
  error: string | null;
}

export function SyncStatusIndicator() {
  const [status, setStatus] = useState<SyncStatus>({
    connected: false,
    lastSync: null,
    pendingChanges: 0,
    error: null,
  });
  const [syncEnabled, setSyncEnabled] = useState(false);

  useEffect(() => {
    const loadStatus = async () => {
      try {
        // Load sync config to check if sync is enabled
        const configResult = await commands.getSyncConfig();
        if (configResult.status === "ok") {
          setSyncEnabled(configResult.data.sync_enabled);
        }

        // Load sync status
        const statusResult = await commands.getSyncStatus();
        if (statusResult.status === "ok") {
          setStatus({
            connected: statusResult.data.connected,
            lastSync: statusResult.data.last_sync,
            pendingChanges: statusResult.data.pending_changes,
            error: statusResult.data.error,
          });
        }
      } catch (error) {
        console.error("Failed to load sync status:", error);
      }
    };

    loadStatus();
    
    // Refresh status every 10 seconds
    const interval = setInterval(loadStatus, 10000);
    return () => clearInterval(interval);
  }, []);

  if (!syncEnabled) {
    return null; // Don't show indicator if sync is disabled
  }

  const getStatusBadge = () => {
    if (status.error) {
      return (
        <Badge variant="destructive" className="text-xs">
          <AlertTriangle className="w-3 h-3 mr-1" />
          Sync Error
        </Badge>
      );
    }
    
    if (status.connected) {
      return (
        <Badge variant="default" className="text-xs">
          <Wifi className="w-3 h-3 mr-1" />
          Synced
        </Badge>
      );
    }
    
    if (status.pendingChanges > 0) {
      return (
        <Badge variant="secondary" className="text-xs">
          <Loader2 className="w-3 h-3 mr-1 animate-spin" />
          Syncing ({status.pendingChanges})
        </Badge>
      );
    }
    
    return (
      <Badge variant="secondary" className="text-xs">
        <WifiOff className="w-3 h-3 mr-1" />
        Offline
      </Badge>
    );
  };

  return getStatusBadge();
}