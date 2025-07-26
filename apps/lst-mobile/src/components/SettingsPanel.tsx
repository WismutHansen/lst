import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { 
  Settings, 
  Wifi, 
  WifiOff, 
  CheckCircle, 
  XCircle, 
  Loader2, 
  Server,
  Shield,
  Smartphone,
  Clock,
  AlertTriangle
} from "lucide-react";
import { commands } from "../bindings";

interface SyncConfig {
  serverUrl: string;
  email: string;
  deviceId: string;
  syncEnabled: boolean;
  syncInterval: number;
  encryptionEnabled: boolean;
}

interface SyncStatus {
  connected: boolean;
  lastSync: string | null;
  pendingChanges: number;
  error: string | null;
}

interface AuthState {
  step: "idle" | "requesting" | "verifying" | "authenticated";
  email: string;
  password: string;
  token: string;
  error: string | null;
}

export function SettingsPanel() {
  const [config, setConfig] = useState<SyncConfig>({
    serverUrl: "",
    email: "",
    deviceId: "",
    syncEnabled: false,
    syncInterval: 30,
    encryptionEnabled: true,
  });

  const [status, setStatus] = useState<SyncStatus>({
    connected: false,
    lastSync: null,
    pendingChanges: 0,
    error: null,
  });

  const [auth, setAuth] = useState<AuthState>({
    step: "idle",
    email: "",
    password: "",
    token: "",
    error: null,
  });

  const [loading, setLoading] = useState(false);

  // Load initial configuration
  useEffect(() => {
    loadSyncConfig();
    loadSyncStatus();
  }, []);

  const loadSyncConfig = async () => {
    try {
      const result = await commands.getSyncConfig();
      if (result.status === "ok") {
        setConfig({
          serverUrl: result.data.server_url,
          email: result.data.email,
          deviceId: result.data.device_id,
          syncEnabled: result.data.sync_enabled,
          syncInterval: result.data.sync_interval,
          encryptionEnabled: result.data.encryption_enabled,
        });
      }
    } catch (error) {
      console.error("Failed to load sync config:", error);
    }
  };

  const loadSyncStatus = async () => {
    try {
      const result = await commands.getSyncStatus();
      if (result.status === "ok") {
        setStatus({
          connected: result.data.connected,
          lastSync: result.data.last_sync,
          pendingChanges: result.data.pending_changes,
          error: result.data.error,
        });
      }
    } catch (error) {
      console.error("Failed to load sync status:", error);
    }
  };

  const handleServerUrlChange = (url: string) => {
    setConfig(prev => ({ ...prev, serverUrl: url }));
  };

  const handleEmailChange = (email: string) => {
    setConfig(prev => ({ ...prev, email }));
    setAuth(prev => ({ ...prev, email }));
  };

  const requestAuthToken = async () => {
    if (!config.email || !config.serverUrl) {
      setAuth(prev => ({ ...prev, error: "Please enter email and server URL" }));
      return;
    }

    setAuth(prev => ({ ...prev, step: "requesting", error: null }));
    setLoading(true);

    try {
      // First save the server URL to config so it's available for verification
      await saveSyncConfig({ ...config, serverUrl: config.serverUrl });
      
      const result = await commands.requestAuthToken(
        config.email, 
        config.serverUrl, 
        auth.password || undefined
      );
      if (result.status === "ok") {
        setAuth(prev => ({ ...prev, step: "verifying" }));
      } else {
        setAuth(prev => ({ ...prev, step: "idle", error: result.error }));
      }
    } catch (error) {
      setAuth(prev => ({ 
        ...prev, 
        step: "idle", 
        error: error instanceof Error ? error.message : "Failed to request token" 
      }));
    } finally {
      setLoading(false);
    }
  };

  const verifyAuthToken = async () => {
    if (!auth.token) {
      setAuth(prev => ({ ...prev, error: "Please enter the verification token" }));
      return;
    }

    setLoading(true);

    try {
      const result = await commands.verifyAuthToken(config.email, auth.token);
      if (result.status === "ok") {
        setAuth(prev => ({ ...prev, step: "authenticated", error: null }));
        setConfig(prev => ({ ...prev, syncEnabled: true }));
        await saveSyncConfig({ ...config, syncEnabled: true });
      } else {
        setAuth(prev => ({ ...prev, error: result.error }));
      }
    } catch (error) {
      setAuth(prev => ({ 
        ...prev, 
        error: error instanceof Error ? error.message : "Failed to verify token" 
      }));
      setLoading(false);
    }
  };

  const saveSyncConfig = async (newConfig: SyncConfig) => {
    try {
      const result = await commands.saveSyncConfig({
        server_url: newConfig.serverUrl,
        email: newConfig.email,
        device_id: newConfig.deviceId,
        sync_enabled: newConfig.syncEnabled,
        sync_interval: newConfig.syncInterval,
        encryption_enabled: newConfig.encryptionEnabled,
      });
      if (result.status === "ok") {
        setConfig(newConfig);
      }
    } catch (error) {
      console.error("Failed to save sync config:", error);
    }
  };

  const toggleSync = async (enabled: boolean) => {
    try {
      const result = await commands.toggleSync(enabled);
      if (result.status === "ok") {
        const newConfig = { ...config, syncEnabled: enabled };
        await saveSyncConfig(newConfig);
      }
    } catch (error) {
      console.error("Failed to toggle sync:", error);
    }
  };

  const testConnection = async () => {
    setLoading(true);
    try {
      const result = await commands.testSyncConnection();
      if (result.status === "ok") {
        setStatus(prev => ({ ...prev, error: null }));
        alert("Connection test successful: " + result.data);
      } else {
        setStatus(prev => ({ ...prev, error: result.error }));
        alert("Connection test failed: " + result.error);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : "Connection test failed";
      setStatus(prev => ({ ...prev, error: errorMsg }));
      alert("Connection test failed: " + errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const resetAuth = () => {
    setAuth({
      step: "idle",
      email: config.email,
      password: "",
      token: "",
      error: null,
    });
    setConfig(prev => ({ ...prev, syncEnabled: false }));
  };

  const getConnectionStatusBadge = () => {
    if (!config.syncEnabled) {
      return <Badge variant="secondary"><WifiOff className="w-3 h-3 mr-1" />Disabled</Badge>;
    }
    
    if (status.connected) {
      return <Badge variant="default" className="bg-green-600"><CheckCircle className="w-3 h-3 mr-1" />Connected</Badge>;
    }
    
    if (status.error) {
      return <Badge variant="destructive"><XCircle className="w-3 h-3 mr-1" />Error</Badge>;
    }
    
    return <Badge variant="secondary"><Loader2 className="w-3 h-3 mr-1 animate-spin" />Connecting</Badge>;
  };

  return (
    <div className="p-6 space-y-6 max-w-2xl mx-auto">
      <div className="flex items-center gap-2 mb-6">
        <Settings className="w-5 h-5" />
        <h1 className="text-xl font-semibold">Settings</h1>
      </div>

      {/* Sync Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="w-4 h-4" />
            Sync Configuration
          </CardTitle>
          <CardDescription>
            Connect to a lst-server instance to sync your lists and notes across devices
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Connection Status */}
          <div className="flex items-center justify-between">
            <Label>Connection Status</Label>
            {getConnectionStatusBadge()}
          </div>

          <Separator />

          {/* Server URL */}
          <div className="space-y-2">
            <Label htmlFor="serverUrl">Server URL</Label>
            <Input
              id="serverUrl"
              placeholder="ws://your-server:5673/api/sync"
              value={config.serverUrl}
              onChange={(e) => handleServerUrlChange(e.target.value)}
              disabled={config.syncEnabled}
            />
          </div>

          {/* Email */}
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              placeholder="your@email.com"
              value={config.email}
              onChange={(e) => handleEmailChange(e.target.value)}
              disabled={config.syncEnabled}
            />
          </div>

          {/* Password (for demo - in production this might be handled differently) */}
          {!config.syncEnabled && (
            <div className="space-y-2">
              <Label htmlFor="password">Password (Optional)</Label>
              <Input
                id="password"
                type="password"
                placeholder="Leave empty for demo mode"
                value={auth.password}
                onChange={(e) => setAuth(prev => ({ ...prev, password: e.target.value }))}
                disabled={config.syncEnabled}
              />
              <p className="text-xs text-muted-foreground">
                For demo purposes. In production, this would be handled securely.
              </p>
            </div>
          )}

          {/* Authentication Flow */}
          {!config.syncEnabled && (
            <div className="space-y-4 p-4 border rounded-lg bg-muted/50">
              <h4 className="font-medium">Authentication</h4>
              
              {auth.step === "idle" && (
                <Button 
                  onClick={requestAuthToken} 
                  disabled={loading || !config.email || !config.serverUrl}
                  className="w-full"
                >
                  {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                  Request Authentication Token
                </Button>
              )}

              {auth.step === "requesting" && (
                <div className="text-center">
                  <Loader2 className="w-6 h-6 mx-auto mb-2 animate-spin" />
                  <p className="text-sm text-muted-foreground">Requesting authentication token...</p>
                </div>
              )}

              {auth.step === "verifying" && (
                <div className="space-y-3">
                  <p className="text-sm text-muted-foreground">
                    Check your email for the verification token and enter it below:
                  </p>
                  <Input
                    placeholder="TOKEN-FROM-EMAIL"
                    value={auth.token}
                    onChange={(e) => setAuth(prev => ({ ...prev, token: e.target.value.toUpperCase() }))}
                  />
                  <div className="flex gap-2">
                    <Button onClick={verifyAuthToken} disabled={loading || !auth.token} className="flex-1">
                      {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                      Verify Token
                    </Button>
                    <Button variant="outline" onClick={resetAuth}>Cancel</Button>
                  </div>
                </div>
              )}

              {auth.step === "authenticated" && (
                <div className="text-center text-green-600">
                  <CheckCircle className="w-6 h-6 mx-auto mb-2" />
                  <p className="text-sm">Successfully authenticated! Sync is now enabled.</p>
                </div>
              )}

              {auth.error && (
                <Alert variant="destructive">
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription>{auth.error}</AlertDescription>
                </Alert>
              )}
            </div>
          )}

          {/* Sync Toggle */}
          {config.syncEnabled && (
            <div className="flex items-center justify-between">
              <div>
                <Label>Enable Sync</Label>
                <p className="text-sm text-muted-foreground">Automatically sync changes with server</p>
              </div>
              <Switch
                checked={config.syncEnabled}
                onCheckedChange={toggleSync}
              />
            </div>
          )}

          {/* Test Connection & Reset Authentication */}
          {config.syncEnabled && (
            <div className="space-y-2">
              <Button 
                variant="outline" 
                onClick={testConnection} 
                disabled={loading}
                className="w-full"
              >
                {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                Test Connection
              </Button>
              <Button variant="outline" onClick={resetAuth} className="w-full">
                Reset Authentication
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Sync Status */}
      {config.syncEnabled && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Wifi className="w-4 h-4" />
              Sync Status
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <Label className="text-sm text-muted-foreground">Last Sync</Label>
                <p className="text-sm font-medium">
                  {status.lastSync ? new Date(status.lastSync).toLocaleString() : "Never"}
                </p>
              </div>
              <div>
                <Label className="text-sm text-muted-foreground">Pending Changes</Label>
                <p className="text-sm font-medium">{status.pendingChanges}</p>
              </div>
            </div>

            {status.error && (
              <Alert variant="destructive">
                <AlertTriangle className="h-4 w-4" />
                <AlertDescription>{status.error}</AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>
      )}

      {/* Device Information */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Smartphone className="w-4 h-4" />
            Device Information
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label className="text-sm text-muted-foreground">Device ID</Label>
            <p className="text-sm font-mono bg-muted p-2 rounded">
              {config.deviceId || "Not configured"}
            </p>
          </div>
          
          <div className="flex items-center gap-2">
            <Shield className="w-4 h-4 text-green-600" />
            <span className="text-sm">End-to-end encryption enabled</span>
          </div>
        </CardContent>
      </Card>

      {/* Advanced Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="w-4 h-4" />
            Advanced Settings
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="syncInterval">Sync Interval (seconds)</Label>
            <Input
              id="syncInterval"
              type="number"
              min="10"
              max="300"
              value={config.syncInterval}
              onChange={(e) => setConfig(prev => ({ 
                ...prev, 
                syncInterval: parseInt(e.target.value) || 30 
              }))}
            />
            <p className="text-xs text-muted-foreground">
              How often to check for changes (10-300 seconds)
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}