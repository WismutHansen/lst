
import React from 'react';
import { SyncStatus as SyncStatusType } from '@/types/List';
import { Badge } from '@/components/ui/badge';
import { Wifi, WifiOff, RefreshCw, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SyncStatusProps {
  status: SyncStatusType;
}

const SyncStatus = ({ status }: SyncStatusProps) => {
  const getStatusInfo = () => {
    if (status.error) {
      return {
        icon: AlertCircle,
        label: 'Error',
        variant: 'destructive' as const,
        color: 'text-destructive',
      };
    }

    if (status.syncing) {
      return {
        icon: RefreshCw,
        label: 'Syncing',
        variant: 'secondary' as const,
        color: 'text-blue-600',
        animate: true,
      };
    }

    if (status.connected) {
      return {
        icon: Wifi,
        label: 'Connected',
        variant: 'default' as const,
        color: 'text-green-600',
      };
    }

    return {
      icon: WifiOff,
      label: 'Offline',
      variant: 'outline' as const,
      color: 'text-muted-foreground',
    };
  };

  const { icon: Icon, label, variant, color, animate } = getStatusInfo();

  return (
    <Badge variant={variant} className="gap-1">
      <Icon
        size={12}
        className={cn(color, animate && 'animate-spin')}
      />
      {label}
      {status.lastSync && (
        <span className="text-xs opacity-75">
          • {status.lastSync.toLocaleTimeString()}
        </span>
      )}
    </Badge>
  );
};

export default SyncStatus;
