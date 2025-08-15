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
  AlertTriangle,
  Palette
} from "lucide-react";
import { commands } from "../bindings";
import { MobileThemeSelector } from "./MobileThemeSelector";

interface SyncConfig {
  serverUrl: string;
  email: string;
  deviceId: string;
  syncEnabled: boolean;
  syncInterval: number;
  encryptionEnabled: boolean;
}

interface ServerConnection {
  ip: string;
  port: string;
}

interface SyncStatus {
  connected: boolean;
  lastSync: string | null;
  pendingChanges: number;
  error: string | null;
}

interface AuthState {
  mode: "choose" | "register" | "login";
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

  const [serverConnection, setServerConnection] = useState<ServerConnection>({
    ip: "",
    port: "5673",
  });

  const [status, setStatus] = useState<SyncStatus>({
    connected: false,
    lastSync: null,
    pendingChanges: 0,
    error: null,
  });

  const [auth, setAuth] = useState<AuthState>({
    mode: "choose",
    step: "idle",
    email: "",
    password: "",
    token: "",
    error: null,
  });

  const [loading, setLoading] = useState(false);
  const [serverConfigSaved, setServerConfigSaved] = useState(false);

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

        // Parse server URL to extract IP and port
        parseServerUrl(result.data.server_url);
      }
    } catch (error) {
      console.error("Failed to load sync config:", error);
    }
  };

  const parseServerUrl = (url: string) => {
    if (!url) return;

    try {
      // Handle URLs like "ws://192.168.1.100:5673/api/sync"
      const urlObj = new URL(url);
      setServerConnection({
        ip: urlObj.hostname,
        port: urlObj.port || "5673",
      });
    } catch (error) {
      console.error("Failed to parse server URL:", error);
    }
  };

  const constructServerUrl = (ip: string, port: string): string => {
    if (!ip.trim()) return "";
    const cleanPort = port.trim() || "5673";
    return `ws://${ip.trim()}:${cleanPort}/api/sync`;
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

  const handleServerConnectionChange = (field: keyof ServerConnection, value: string) => {
    const newConnection = { ...serverConnection, [field]: value };
    setServerConnection(newConnection);

    // Update the full server URL in config
    const fullUrl = constructServerUrl(newConnection.ip, newConnection.port);
    setConfig(prev => ({ ...prev, serverUrl: fullUrl }));
    
    // Reset saved state when server settings change
    setServerConfigSaved(false);
  };

  const handleEmailChange = (email: string) => {
    setConfig(prev => ({ ...prev, email }));
    setAuth(prev => ({ ...prev, email }));
    // Reset saved state when email changes
    setServerConfigSaved(false);
  };

  const requestAuthToken = async () => {
    if (!config.email || !serverConnection.ip) {
      setAuth(prev => ({ ...prev, error: "Please enter email and server IP address" }));
      return;
    }

    setAuth(prev => ({ ...prev, step: "requesting", error: null }));
    setLoading(true);

    try {
      // Construct the full server URL
      const fullServerUrl = constructServerUrl(serverConnection.ip, serverConnection.port);
      const updatedConfig = { ...config, serverUrl: fullServerUrl };

      // First save the server URL to config so it's available for verification
      await saveSyncConfig(updatedConfig);

      const result = await commands.requestAuthToken(
        config.email,
        fullServerUrl,
        auth.password || null
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

  const secureLogin = async () => {
    if (!config.email || !auth.token || !auth.password || !serverConnection.ip) {
      setAuth(prev => ({ ...prev, error: "Please enter email, password, auth token, and server IP address" }));
      return;
    }

    setAuth(prev => ({ ...prev, step: "requesting", error: null }));
    setLoading(true);

    try {
      // Construct the full server URL
      const fullServerUrl = constructServerUrl(serverConnection.ip, serverConnection.port);
      const updatedConfig = { ...config, serverUrl: fullServerUrl };

      // First save the server URL to config so it's available
      await saveSyncConfig(updatedConfig);

      const result = await commands.secureLogin(
        config.email,
        auth.token,
        auth.password,
        fullServerUrl
      );
      if (result.status === "ok") {
        setAuth(prev => ({ ...prev, step: "authenticated", error: null }));
        setConfig(prev => ({ ...prev, syncEnabled: true }));
        await saveSyncConfig({ ...config, syncEnabled: true });
      } else {
        setAuth(prev => ({ ...prev, step: "idle", error: result.error }));
      }
    } catch (error) {
      setAuth(prev => ({
        ...prev,
        step: "idle",
        error: error instanceof Error ? error.message : "Failed to login"
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
      const result = await commands.verifyAuthToken(config.email, auth.token, config.serverUrl);
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

  const saveServerConfig = async () => {
    if (!serverConnection.ip || !config.email) {
      alert("Please enter both server IP and email before saving");
      return;
    }

    setLoading(true);
    try {
      const serverUrl = `ws://${serverConnection.ip}:${serverConnection.port || "5673"}/api/sync`;
      const deviceId = config.deviceId || `mobile-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      
      const result = await commands.saveSyncConfig({
        server_url: serverUrl,
        email: config.email,
        device_id: deviceId,
        sync_enabled: false, // Don't enable sync yet, just save the config
        sync_interval: config.syncInterval,
        encryption_enabled: config.encryptionEnabled,
      });
      
      if (result.status === "ok") {
        setConfig(prev => ({ 
          ...prev, 
          serverUrl,
          deviceId 
        }));
        setServerConfigSaved(true);
        alert("Server configuration saved successfully! You can now request and verify your auth token.");
      } else {
        alert("Failed to save server configuration: " + result.error);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : "Failed to save server configuration";
      alert("Failed to save server configuration: " + errorMsg);
    } finally {
      setLoading(false);
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

  const triggerSync = async () => {
    setLoading(true);
    try {
      const result = await commands.triggerSync();
      if (result.status === "ok") {
        setStatus(prev => ({ ...prev, error: null }));
        alert("Sync successful: " + result.data);
      } else {
        setStatus(prev => ({ ...prev, error: result.error }));
        alert("Sync failed: " + result.error);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : "Sync failed";
      setStatus(prev => ({ ...prev, error: errorMsg }));
      alert("Sync failed: " + errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const debugSyncStatus = async () => {
    setLoading(true);
    try {
      const result = await commands.debugSyncStatus();
      if (result.status === "ok") {
        console.log("🔍 DEBUG Sync Status:", result.data);
        alert("Debug info logged to console:\n\n" + result.data);
      } else {
        console.error("🔍 DEBUG Sync Status Error:", result.error);
        alert("Debug failed: " + result.error);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : "Debug failed";
      console.error("🔍 DEBUG Sync Status Exception:", errorMsg);
      alert("Debug failed: " + errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const resetAuth = () => {
    setAuth({
      mode: "choose",
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
      return <Badge variant="secondary"><WifiOff className="text-muted-foreground w-3 h-3 mr-1" />Disabled</Badge>;
    }

    if (status.connected) {
      return <Badge variant="default"><CheckCircle className="w-3 h-3 mr-1" />Connected</Badge>;
    }

    if (status.error) {
      return <Badge variant="destructive"><XCircle className="text-muted-foreground w-3 h-3 mr-1" />Error</Badge>;
    }

    return <Badge variant="secondary"><Loader2 className="text-muted-foreground w-3 h-3 mr-1 animate-spin" />Connecting</Badge>;
  };

  return (
    <div className="p-4 space-y-6 w-full h-full overflow-y-auto">
      <div className="flex items-center gap-2 mb-6">
        <Settings className="w-5 h-5" />
        <h1 className="text-xl font-semibold">Settings</h1>
      </div>

      {/* Sync Configuration */}
      <Card className="bg-muted/20">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="w-4 h-4" />
            Sync Configuration
          </CardTitle>
          <CardDescription className="text-primary/40">
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

          {/* Server Connection */}
          <div className="space-y-2">
            <Label>Server Connection</Label>
            <div className="flex gap-2">
              <div className="flex-1">
                <Label htmlFor="serverIp" className="text-xs text-muted-foreground">IP Address</Label>
                <Input
                  id="serverIp"
                  placeholder="192.168.1.100"
                  value={serverConnection.ip}
                  onChange={(e) => handleServerConnectionChange("ip", e.target.value)}
                  disabled={config.syncEnabled}
                />
              </div>
              <div className="w-20">
                <Label htmlFor="serverPort" className="text-xs text-muted-foreground">Port</Label>
                <Input
                  id="serverPort"
                  placeholder="5673"
                  value={serverConnection.port}
                  onChange={(e) => handleServerConnectionChange("port", e.target.value)}
                  disabled={config.syncEnabled}
                />
              </div>
            </div>
            {serverConnection.ip && (
              <p className="text-xs text-muted-foreground">
                Will connect to: ws://{serverConnection.ip}:{serverConnection.port || "5673"}/api/sync
              </p>
            )}
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

          {/* Save Server Configuration - moved here to show it requires all above fields */}
          {!config.syncEnabled && (
            <Button
              variant={serverConfigSaved ? "default" : "outline"}
              onClick={saveServerConfig}
              disabled={loading || !serverConnection.ip || !config.email}
              className="w-full"
            >
              {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
              {serverConfigSaved && !loading && <CheckCircle className="w-4 h-4 mr-2" />}
              {serverConfigSaved ? "Server Configuration Saved" : "Save Server Configuration"}
            </Button>
          )}

          {/* Authentication Flow */}
          {!config.syncEnabled && (
            <div className="space-y-4 p-4 border rounded-lg bg-muted/50">
              <h4 className="font-medium">Authentication</h4>
              {!serverConfigSaved && (
                <Alert>
                  <AlertDescription>
                    Please save your server configuration above before authenticating.
                  </AlertDescription>
                </Alert>
              )}

              {/* Choose between Register and Login */}
              {auth.mode === "choose" && auth.step === "idle" && (
                <div className="space-y-3">
                  <p className="text-sm text-muted-foreground">Choose your authentication method:</p>
                  <div className="grid grid-cols-2 gap-3">
                    <Button
                      onClick={() => setAuth(prev => ({ ...prev, mode: "register" }))}
                      disabled={!serverConfigSaved}
                      variant="outline"
                      className="h-auto flex-col p-4"
                    >
                      <span className="font-medium">Register New Account</span>
                      <span className="text-xs text-muted-foreground mt-1">First-time users only</span>
                    </Button>
                    <Button
                      onClick={() => setAuth(prev => ({ ...prev, mode: "login" }))}
                      disabled={!serverConfigSaved}
                      variant="outline"
                      className="h-auto flex-col p-4"
                    >
                      <span className="font-medium">Login Existing</span>
                      <span className="text-xs text-muted-foreground mt-1">I have an auth token</span>
                    </Button>
                  </div>
                </div>
              )}

              {/* Register Flow */}
              {auth.mode === "register" && auth.step === "idle" && (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 mb-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setAuth(prev => ({ ...prev, mode: "choose" }))}
                      className="p-1 h-auto"
                    >
                      ← Back
                    </Button>
                    <span className="font-medium">Register New Account</span>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="register-password">Account Password</Label>
                    <Input
                      id="register-password"
                      type="password"
                      placeholder="Enter password for new account"
                      value={auth.password}
                      onChange={(e) => setAuth(prev => ({ ...prev, password: e.target.value }))}
                    />
                  </div>
                  <Button
                    onClick={requestAuthToken}
                    disabled={loading || !config.email || !auth.password || !serverConnection.ip}
                    className="w-full"
                  >
                    {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                    Register Account
                  </Button>
                </div>
              )}

              {/* Login Flow */}
              {auth.mode === "login" && auth.step === "idle" && (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 mb-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setAuth(prev => ({ ...prev, mode: "choose" }))}
                      className="p-1 h-auto"
                    >
                      ← Back
                    </Button>
                    <span className="font-medium">Login with Existing Account</span>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="login-password">Account Password</Label>
                    <Input
                      id="login-password"
                      type="password"
                      placeholder="Enter your account password"
                      value={auth.password}
                      onChange={(e) => setAuth(prev => ({ ...prev, password: e.target.value }))}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="auth-token">Auth Token</Label>
                    <Input
                      id="auth-token"
                      placeholder="LEAF-DAWN-LARK-7114"
                      value={auth.token}
                      onChange={(e) => setAuth(prev => ({ ...prev, token: e.target.value.toUpperCase() }))}
                    />
                    <p className="text-xs text-muted-foreground">
                      Enter the auth token from server console or QR code
                    </p>
                  </div>
                  <Button
                    onClick={secureLogin}
                    disabled={loading || !config.email || !auth.password || !auth.token || !serverConnection.ip}
                    className="w-full"
                  >
                    {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                    Login Securely
                  </Button>
                </div>
              )}

              {/* Processing States */}
              {auth.step === "requesting" && (
                <div className="text-center">
                  <Loader2 className="w-6 h-6 mx-auto mb-2 animate-spin" />
                  <p className="text-sm text-muted-foreground break-words">
                    {auth.mode === "register" ? "Registering account..." : "Logging in..."}
                  </p>
                </div>
              )}

              {/* Register verification step */}
              {auth.mode === "register" && auth.step === "verifying" && (
                <div className="space-y-3">
                  <p className="text-sm text-muted-foreground break-words">
                    🔐 Registration successful! The auth token is displayed on the SERVER CONSOLE.
                    Check the server logs or scan the QR code, then enter the token below:
                  </p>
                  <Input
                    placeholder="LEAF-DAWN-LARK-7114"
                    value={auth.token}
                    onChange={(e) => setAuth(prev => ({ ...prev, token: e.target.value.toUpperCase() }))}
                  />
                  <div className="flex gap-2">
                    <Button onClick={verifyAuthToken} disabled={loading || !auth.token} className="flex-1">
                      {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                      Verify Token
                    </Button>
                    <Button variant="outline" onClick={() => setAuth(prev => ({ ...prev, mode: "choose", step: "idle" }))}>
                      Cancel
                    </Button>
                  </div>
                </div>
              )}

              {/* Success State */}
              {auth.step === "authenticated" && (
                <div className="text-center text-primary">
                  <CheckCircle className="w-6 h-6 mx-auto mb-2" />
                  <p className="text-sm">Successfully authenticated! Secure sync is now enabled.</p>
                </div>
              )}

              {/* Error Display */}
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
                <p className="text-sm text-muted-foreground break-words">Automatically sync changes with server</p>              </div>
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
              <Button
                variant="outline"
                onClick={triggerSync}
                disabled={loading}
                className="w-full"
              >
                {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                Trigger Sync
              </Button>
              <Button
                variant="outline"
                onClick={debugSyncStatus}
                disabled={loading}
                className="w-full"
              >
                {loading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                🔍 Debug Sync
              </Button>
              <Button variant="outline" onClick={resetAuth} className="w-full">
                Reset Authentication
              </Button>
            </div>
          )}        </CardContent>
      </Card>

      {/* Theme Configuration */}
      <Card className="flex content-center bg-muted/20">
        <CardHeader>
          <CardTitle className="flex self-center">
            <div>
              <Palette className="self-center w-4 h-4 mr-2" />
            </div>
            <div className="self-center mr-2">
              Theme
            </div>
          </CardTitle>
        </CardHeader>
        <CardContent className="ml-2">
          <MobileThemeSelector />
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
      <Card className="bg-muted/20">
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
            <Shield className="w-4 h-4 text-primary" />
            <span className="text-sm">End-to-end encryption enabled</span>
          </div>
        </CardContent>
      </Card>

      {/* Advanced Settings */}
      <Card className="bg-muted/20">
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
            <p className="text-xs text-foreground/30">
              How often to check for changes (10-300 seconds)
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
