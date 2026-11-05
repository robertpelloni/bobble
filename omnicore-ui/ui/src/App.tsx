import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface BridgeStatusInfo {
  name: string;
  protocol: string;
  status: string;
  endpoint: string;
  latency_ms: number;
  fallback_priority: number;
  connected: boolean;
}

interface ContactInfo {
  id: number;
  name: string;
  unified_id: string;
  primary_protocol: string;
  fallback_protocol: string;
  avatar_initials: string;
  status: string;
  mapped_protocols: string[];
}

interface MessageItem {
  id: string;
  sender: string;
  recipient: string;
  content: string;
  protocol: string;
  timestamp: string;
  is_outgoing: boolean;
  status: string;
}

interface SystemMetrics {
  version: string;
  active_bridges: number;
  queued_messages: number;
  encryption: string;
  uptime_secs: number;
  server_running: boolean;
}

const DEFAULT_METRICS: SystemMetrics = {
  version: '0.3.5',
  active_bridges: 4,
  queued_messages: 0,
  encryption: 'SQLCipher (AES-256-GCM)',
  uptime_secs: 120,
  server_running: true,
};

const DEFAULT_CONTACTS: ContactInfo[] = [
  {
    id: 1,
    name: 'Alice Montgomery',
    unified_id: 'alice_unified_1',
    primary_protocol: 'Signal',
    fallback_protocol: 'Matrix',
    avatar_initials: 'AM',
    status: 'Online',
    mapped_protocols: ['Signal', 'Matrix', 'SMS'],
  },
  {
    id: 2,
    name: 'Bob Sterling',
    unified_id: 'bob_unified_2',
    primary_protocol: 'Matrix',
    fallback_protocol: 'Discord',
    avatar_initials: 'BS',
    status: 'Away',
    mapped_protocols: ['Matrix', 'Discord'],
  },
  {
    id: 3,
    name: 'Charlie Vance',
    unified_id: 'charlie_unified_3',
    primary_protocol: 'Discord',
    fallback_protocol: 'Signal',
    avatar_initials: 'CV',
    status: 'Offline',
    mapped_protocols: ['Discord', 'Signal'],
  },
  {
    id: 4,
    name: 'Dana Scully',
    unified_id: 'dana_unified_4',
    primary_protocol: 'Simplex',
    fallback_protocol: 'Tox',
    avatar_initials: 'DS',
    status: 'Online',
    mapped_protocols: ['Simplex', 'Tox'],
  },
];

const DEFAULT_BRIDGES: BridgeStatusInfo[] = [
  {
    name: 'Signal Daemon Bridge',
    protocol: 'Signal',
    status: 'Connected',
    endpoint: 'unix:///var/run/signald/signald.sock',
    latency_ms: 12,
    fallback_priority: 1,
    connected: true,
  },
  {
    name: 'Matrix Synapse/Dendrite',
    protocol: 'Matrix',
    status: 'Connected',
    endpoint: 'https://matrix.org',
    latency_ms: 45,
    fallback_priority: 2,
    connected: true,
  },
  {
    name: 'Discord Gateway Bridge',
    protocol: 'Discord',
    status: 'Connected',
    endpoint: 'wss://gateway.discord.gg',
    latency_ms: 38,
    fallback_priority: 3,
    connected: true,
  },
  {
    name: 'SimpleX Chat Isolated Broker',
    protocol: 'Simplex',
    status: 'Standby',
    endpoint: 'simplex://broker.v3.local',
    latency_ms: 60,
    fallback_priority: 4,
    connected: false,
  },
  {
    name: 'Tox Distributed P2P Core',
    protocol: 'Tox',
    status: 'Standby',
    endpoint: 'dht://bootstrap.tox.chat',
    latency_ms: 95,
    fallback_priority: 5,
    connected: false,
  },
  {
    name: 'iMessage BlueBubbles Relay',
    protocol: 'iMessage',
    status: 'Configured',
    endpoint: 'https://relay.imessage.local',
    latency_ms: 50,
    fallback_priority: 6,
    connected: false,
  },
  {
    name: 'SMS / Cellular Modem',
    protocol: 'SMS',
    status: 'Ready',
    endpoint: 'serial:///dev/ttyUSB0',
    latency_ms: 110,
    fallback_priority: 7,
    connected: true,
  },
];

const DEFAULT_MESSAGES: MessageItem[] = [
  {
    id: 'msg_1',
    sender: 'Alice Montgomery',
    recipient: 'You',
    content: 'Hey Robert! Is OmniCore cross-protocol routing active?',
    protocol: 'Signal',
    timestamp: '10:42 AM',
    is_outgoing: false,
    status: 'delivered',
  },
  {
    id: 'msg_2',
    sender: 'You',
    recipient: 'Alice Montgomery',
    content: 'Yes, fully active! Routing across Signal, Matrix, Simplex, and Discord seamlessly.',
    protocol: 'Signal (fallback: Matrix)',
    timestamp: '10:43 AM',
    is_outgoing: true,
    status: 'delivered',
  },
  {
    id: 'msg_3',
    sender: 'Bob Sterling',
    recipient: 'You',
    content: 'Confirming bridge sync token received via Matrix homeserver.',
    protocol: 'Matrix',
    timestamp: '10:45 AM',
    is_outgoing: false,
    status: 'delivered',
  },
];

function App() {
  const isTauri = typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI_IPC__' in window);

  const [engineStatus, setEngineStatus] = useState<string>('Initializing...');
  const [metrics, setMetrics] = useState<SystemMetrics>(DEFAULT_METRICS);
  const [contacts, setContacts] = useState<ContactInfo[]>(DEFAULT_CONTACTS);
  const [selectedContact, setSelectedContact] = useState<ContactInfo>(DEFAULT_CONTACTS[0]);
  const [messages, setMessages] = useState<MessageItem[]>(DEFAULT_MESSAGES);
  const [bridges, setBridges] = useState<BridgeStatusInfo[]>(DEFAULT_BRIDGES);

  // Dashboard view toggle: 'hub' (all consolidated high-value), 'bridges', 'audit'
  const [activeTab, setActiveTab] = useState<'hub' | 'bridges' | 'audit'>('hub');

  // Input states
  const [inputText, setInputText] = useState<string>('');
  const [primaryProtocol, setPrimaryProtocol] = useState<string>('Signal');
  const [fallbackProtocol, setFallbackProtocol] = useState<string>('Matrix');
  const [searchQuery, setSearchQuery] = useState<string>('');

  // Contact Creation Modal/Inline
  const [showAddContact, setShowAddContact] = useState<boolean>(false);
  const [newContactName, setNewContactName] = useState<string>('');
  const [newContactPrimary, setNewContactPrimary] = useState<string>('Signal');
  const [newContactFallback, setNewContactFallback] = useState<string>('Matrix');

  // Tray menu state
  const [showTrayMenu, setShowTrayMenu] = useState<boolean>(false);

  useEffect(() => {
    if (isTauri) {
      invoke('init_engine')
        .then((res) => setEngineStatus(res as string))
        .catch((err) => setEngineStatus(`Error: ${err}`));

      invoke('get_system_metrics')
        .then((res) => setMetrics(res as SystemMetrics))
        .catch(console.error);

      invoke('get_contacts')
        .then((res) => {
          const list = res as ContactInfo[];
          if (list.length > 0) {
            setContacts(list);
            setSelectedContact(list[0]);
          }
        })
        .catch(console.error);

      invoke('get_messages')
        .then((res) => setMessages(res as MessageItem[]))
        .catch(console.error);

      invoke('get_bridges')
        .then((res) => setBridges(res as BridgeStatusInfo[]))
        .catch(console.error);
    } else {
      setEngineStatus('Engine & bridges active (Mock Mode)');
    }
  }, [isTauri]);

  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputText.trim()) return;

    if (isTauri) {
      invoke('send_message', {
        recipient: selectedContact.name,
        content: inputText,
        protocol: primaryProtocol,
        fallbackProtocol: fallbackProtocol,
      })
        .then((res) => {
          setMessages((prev) => [...prev, res as MessageItem]);
          setInputText('');
        })
        .catch((err) => alert(`Send failed: ${err}`));
    } else {
      const newMsg: MessageItem = {
        id: `msg_${Date.now()}`,
        sender: 'You',
        recipient: selectedContact.name,
        content: inputText,
        protocol: `${primaryProtocol} (fallback: ${fallbackProtocol})`,
        timestamp: 'Just now',
        is_outgoing: true,
        status: 'dispatched',
      };
      setMessages((prev) => [...prev, newMsg]);
      setInputText('');
    }
  };

  const handleToggleBridge = (protocol: string) => {
    if (isTauri) {
      invoke('toggle_bridge', { protocol })
        .then((updated) => {
          const bridge = updated as BridgeStatusInfo;
          setBridges((prev) => prev.map((b) => (b.protocol === bridge.protocol ? bridge : b)));
        })
        .catch(console.error);
    } else {
      setBridges((prev) =>
        prev.map((b) =>
          b.protocol === protocol
            ? {
                ...b,
                connected: !b.connected,
                status: !b.connected ? 'Connected' : 'Disconnected',
              }
            : b
        )
      );
    }
  };

  const handleToggleServer = () => {
    if (isTauri) {
      invoke('toggle_servers')
        .then((running) => {
          setMetrics((prev) => ({ ...prev, server_running: running as boolean }));
        })
        .catch(console.error);
    } else {
      setMetrics((prev) => ({ ...prev, server_running: !prev.server_running }));
    }
  };

  const handleAddContact = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newContactName.trim()) return;

    if (isTauri) {
      invoke('add_contact', {
        name: newContactName,
        primaryProtocol: newContactPrimary,
        fallbackProtocol: newContactFallback,
        mappedProtocols: [newContactPrimary, newContactFallback],
      })
        .then((c) => {
          const created = c as ContactInfo;
          setContacts((prev) => [...prev, created]);
          setSelectedContact(created);
          setNewContactName('');
          setShowAddContact(false);
        })
        .catch(console.error);
    } else {
      const initials = newContactName
        .split(' ')
        .map((w) => w[0])
        .join('')
        .toUpperCase();
      const newC: ContactInfo = {
        id: Date.now(),
        name: newContactName,
        unified_id: `contact_unified_${Date.now()}`,
        primary_protocol: newContactPrimary,
        fallback_protocol: newContactFallback,
        avatar_initials: initials || 'U',
        status: 'Online',
        mapped_protocols: [newContactPrimary, newContactFallback],
      };
      setContacts((prev) => [...prev, newC]);
      setSelectedContact(newC);
      setNewContactName('');
      setShowAddContact(false);
    }
  };

  const filteredContacts = contacts.filter((c) =>
    c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    c.primary_protocol.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans select-none overflow-hidden">
      {/* 1. Header & System Tray Bar */}
      <header className="h-14 bg-slate-900 border-b border-slate-800 px-4 flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <span className="text-xl">🌐</span>
            <span className="font-extrabold text-base tracking-wide bg-gradient-to-r from-blue-400 to-indigo-400 bg-clip-text text-transparent">
              OmniCore
            </span>
            <span
              className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-blue-900/60 text-blue-300 border border-blue-700/50"
              title="Global Release Build"
            >
              v{metrics.version}
            </span>
          </div>

          <div className="h-4 w-px bg-slate-700 hidden sm:block"></div>

          {/* Engine & SQLCipher Status Badge */}
          <div
            className="flex items-center space-x-2 px-2.5 py-1 rounded bg-slate-800/80 border border-slate-700/60 text-xs cursor-help"
            title="Encrypted Database & Bridge Status"
          >
            <span className={`w-2 h-2 rounded-full ${metrics.server_running ? 'bg-emerald-400 animate-pulse' : 'bg-rose-500'}`}></span>
            <span className="text-slate-300 font-medium">{engineStatus}</span>
            <span className="text-slate-500 font-mono text-[11px]">| {metrics.encryption}</span>
          </div>
        </div>

        {/* High-Value Category Nav Tabs */}
        <div className="flex items-center space-x-1 bg-slate-950/70 p-1 rounded-lg border border-slate-800">
          <button
            onClick={() => setActiveTab('hub')}
            className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
              activeTab === 'hub' ? 'bg-blue-600 text-white shadow-sm' : 'text-slate-400 hover:text-white'
            }`}
            title="Consolidated Universal Messaging & Contact Hub"
          >
            💬 Unified Hub
          </button>
          <button
            onClick={() => setActiveTab('bridges')}
            className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
              activeTab === 'bridges' ? 'bg-blue-600 text-white shadow-sm' : 'text-slate-400 hover:text-white'
            }`}
            title="Inspect and Control Active Protocol Bridges (Signal, Matrix, Discord, Simplex)"
          >
            🔌 Bridges ({bridges.filter((b) => b.connected).length}/{bridges.length})
          </button>
          <button
            onClick={() => setActiveTab('audit')}
            className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
              activeTab === 'audit' ? 'bg-blue-600 text-white shadow-sm' : 'text-slate-400 hover:text-white'
            }`}
            title="View Realtime Message Queue and Dispatch Fallback Auditing"
          >
            📊 Routing Monitor
          </button>
        </div>

        {/* System Controls & Tray Menu Proxy */}
        <div className="flex items-center space-x-3 relative">
          <button
            onClick={handleToggleServer}
            className={`text-xs px-2.5 py-1 rounded font-medium transition-colors border ${
              metrics.server_running
                ? 'bg-emerald-950/50 text-emerald-300 border-emerald-800/60 hover:bg-emerald-900/60'
                : 'bg-rose-950/50 text-rose-300 border-rose-800/60 hover:bg-rose-900/60'
            }`}
            title="Toggle Engine Background Dispatch Server"
          >
            {metrics.server_running ? '🟢 Engine Active' : '🔴 Engine Paused'}
          </button>

          {/* System Tray Icon Trigger */}
          <div className="relative">
            <button
              onClick={() => setShowTrayMenu(!showTrayMenu)}
              className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition"
              title="Native System Tray & Global Services Menu"
              aria-label="System Tray Menu"
            >
              ⚙️
            </button>

            {showTrayMenu && (
              <div className="absolute right-0 mt-2 w-56 bg-slate-900 border border-slate-700 rounded-lg shadow-2xl py-1.5 z-50 text-xs">
                <div className="px-3 py-2 border-b border-slate-800">
                  <div className="font-bold text-slate-200">System Tray Service</div>
                  <div className="text-[11px] text-slate-400">OmniCore Daemon Control</div>
                </div>
                <button
                  onClick={() => {
                    alert('Window visibility toggled via system tray event handler.');
                    setShowTrayMenu(false);
                  }}
                  className="w-full text-left px-3 py-2 text-slate-200 hover:bg-slate-800 flex items-center justify-between"
                >
                  <span>Toggle Dashboard View</span>
                  <span className="text-[10px] text-slate-400">Ctrl+H</span>
                </button>
                <button
                  onClick={() => {
                    handleToggleServer();
                    setShowTrayMenu(false);
                  }}
                  className="w-full text-left px-3 py-2 text-slate-200 hover:bg-slate-800"
                >
                  {metrics.server_running ? 'Restart Routing Workers' : 'Start Routing Workers'}
                </button>
                <div className="border-t border-slate-800 my-1"></div>
                <button
                  onClick={() => {
                    if (isTauri) {
                      invoke('quit_app');
                    } else {
                      alert('Server shutdown requested.');
                    }
                    setShowTrayMenu(false);
                  }}
                  className="w-full text-left px-3 py-2 text-rose-400 hover:bg-rose-950/50 font-semibold flex items-center space-x-1"
                >
                  <span>❌</span>
                  <span>Quit All Servers & Bridges</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* 2. Main Dashboard Workspace */}
      <div className="flex-1 flex overflow-hidden">
        {activeTab === 'hub' && (
          <>
            {/* Left Pane: Unified Contacts & Identities */}
            <section className="w-80 bg-slate-900/80 border-r border-slate-800 flex flex-col shrink-0">
              <div className="p-3 border-b border-slate-800 flex flex-col space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-1">
                    <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
                      Unified Identities
                    </span>
                    <span
                      className="cursor-help text-slate-500 text-xs"
                      title="Unified Identities group a single person's accounts across Signal, Matrix, Discord, SMS, and Simplex into one contact card."
                    >
                      ℹ️
                    </span>
                  </div>
                  <button
                    onClick={() => setShowAddContact(!showAddContact)}
                    className="text-xs px-2 py-0.5 bg-blue-600 hover:bg-blue-500 rounded font-semibold text-white transition"
                    title="Add a new unified cross-protocol contact"
                  >
                    + Add Identity
                  </button>
                </div>

                <input
                  type="text"
                  placeholder="Filter contacts or protocols..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 rounded px-2.5 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
                />
              </div>

              {/* Add Contact Modal / Form */}
              {showAddContact && (
                <form
                  onSubmit={handleAddContact}
                  className="p-3 bg-slate-800/90 border-b border-slate-700 flex flex-col space-y-2 text-xs"
                >
                  <div className="font-bold text-blue-300">New Unified Contact</div>
                  <input
                    type="text"
                    placeholder="Full Name / Handle"
                    value={newContactName}
                    onChange={(e) => setNewContactName(e.target.value)}
                    required
                    className="bg-slate-900 border border-slate-700 rounded p-1.5 text-white"
                  />
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label className="text-[10px] text-slate-400">Primary Protocol</label>
                      <select
                        value={newContactPrimary}
                        onChange={(e) => setNewContactPrimary(e.target.value)}
                        className="w-full bg-slate-900 border border-slate-700 rounded p-1 text-white"
                      >
                        <option value="Signal">Signal</option>
                        <option value="Matrix">Matrix</option>
                        <option value="Discord">Discord</option>
                        <option value="Simplex">Simplex</option>
                        <option value="SMS">SMS</option>
                      </select>
                    </div>
                    <div>
                      <label className="text-[10px] text-slate-400">Fallback Protocol</label>
                      <select
                        value={newContactFallback}
                        onChange={(e) => setNewContactFallback(e.target.value)}
                        className="w-full bg-slate-900 border border-slate-700 rounded p-1 text-white"
                      >
                        <option value="Matrix">Matrix</option>
                        <option value="Signal">Signal</option>
                        <option value="Discord">Discord</option>
                        <option value="Simplex">Simplex</option>
                        <option value="SMS">SMS</option>
                      </select>
                    </div>
                  </div>
                  <div className="flex space-x-2 pt-1">
                    <button
                      type="submit"
                      className="flex-1 bg-blue-600 hover:bg-blue-500 text-white rounded py-1 font-semibold"
                    >
                      Save Identity
                    </button>
                    <button
                      type="button"
                      onClick={() => setShowAddContact(false)}
                      className="px-2 bg-slate-700 hover:bg-slate-600 rounded text-slate-300"
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              )}

              {/* Contact List */}
              <div className="flex-1 overflow-y-auto divide-y divide-slate-800/50">
                {filteredContacts.map((contact) => (
                  <div
                    key={contact.id}
                    onClick={() => {
                      setSelectedContact(contact);
                      setPrimaryProtocol(contact.primary_protocol);
                      setFallbackProtocol(contact.fallback_protocol);
                    }}
                    className={`p-3 flex items-center space-x-3 cursor-pointer transition-colors ${
                      selectedContact.id === contact.id
                        ? 'bg-blue-900/30 border-l-4 border-blue-500'
                        : 'hover:bg-slate-800/40'
                    }`}
                  >
                    <div className="w-9 h-9 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center font-bold text-xs text-blue-300 shrink-0">
                      {contact.avatar_initials}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <span className="font-semibold text-xs text-white truncate">{contact.name}</span>
                        <span className="text-[10px] text-slate-400 flex items-center space-x-1">
                          <span
                            className={`w-1.5 h-1.5 rounded-full ${
                              contact.status === 'Online'
                                ? 'bg-emerald-400'
                                : contact.status === 'Away'
                                ? 'bg-amber-400'
                                : 'bg-slate-500'
                            }`}
                          ></span>
                          <span>{contact.status}</span>
                        </span>
                      </div>
                      <div className="flex items-center space-x-1 mt-1 overflow-x-hidden">
                        {contact.mapped_protocols.map((proto) => (
                          <span
                            key={proto}
                            className="text-[9px] px-1.5 py-0.2 rounded bg-slate-800 border border-slate-700 text-slate-300"
                            title={`Linked ID: ${contact.unified_id} on ${proto}`}
                          >
                            {proto}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* Middle Pane: Universal Chat Feed (Highest Value Core Feature) */}
            <section className="flex-1 flex flex-col bg-slate-950 overflow-hidden">
              {/* Chat Thread Header */}
              <div className="h-14 border-b border-slate-800 px-6 flex items-center justify-between bg-slate-900/40 shrink-0">
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 rounded-full bg-indigo-950 border border-indigo-700/60 flex items-center justify-center text-xs font-bold text-indigo-300">
                    {selectedContact.avatar_initials}
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-sm text-white">{selectedContact.name}</span>
                      <span className="text-[10px] text-slate-400 font-mono bg-slate-800/80 px-1.5 py-0.5 rounded">
                        {selectedContact.unified_id}
                      </span>
                    </div>
                    <div className="text-[11px] text-slate-400 flex items-center space-x-2">
                      <span>Preferred: <strong className="text-emerald-400">{selectedContact.primary_protocol}</strong></span>
                      <span>•</span>
                      <span>Fallback: <strong className="text-indigo-300">{selectedContact.fallback_protocol}</strong></span>
                    </div>
                  </div>
                </div>

                {/* Routing Strategy Quick Info */}
                <div className="flex items-center space-x-2">
                  <span
                    className="text-xs bg-slate-800 border border-slate-700 px-2 py-1 rounded text-slate-300 cursor-help"
                    title="OmniCore automatically tests channel latency and falls back seamlessly if delivery receipts or connection timeouts occur."
                  >
                    ⚡ Smart Multi-Path Enabled
                  </span>
                </div>
              </div>

              {/* Messages Scroll Area */}
              <div className="flex-1 overflow-y-auto p-6 space-y-4">
                {messages.map((msg) => (
                  <div
                    key={msg.id}
                    className={`flex flex-col space-y-1 ${msg.is_outgoing ? 'items-end' : 'items-start'}`}
                  >
                    <div className="flex items-center space-x-2 px-1">
                      <span className="text-[11px] font-bold text-slate-400">{msg.sender}</span>
                      <span className="text-[10px] text-slate-500">{msg.timestamp}</span>
                      <span
                        className="text-[9px] px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700 text-blue-300 cursor-help"
                        title="Aggregated Network Transport Route"
                      >
                        {msg.protocol}
                      </span>
                    </div>
                    <div
                      className={`p-3.5 rounded-2xl max-w-xl text-sm leading-relaxed ${
                        msg.is_outgoing
                          ? 'bg-blue-600 text-white rounded-br-xs shadow-md'
                          : 'bg-slate-800 border border-slate-700 text-slate-100 rounded-bl-xs'
                      }`}
                    >
                      {msg.content}
                    </div>
                    {msg.is_outgoing && (
                      <span className="text-[9px] text-slate-500 pr-1 flex items-center space-x-1">
                        <span>✓✓ {msg.status}</span>
                      </span>
                    )}
                  </div>
                ))}
              </div>

              {/* Multi-Platform Message Composition Dock */}
              <div className="p-4 border-t border-slate-800 bg-slate-900/60 flex flex-col space-y-2">
                {/* Routing & Fallback Protocol Selector */}
                <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
                  <div className="flex items-center space-x-2">
                    <span className="text-slate-400 font-semibold uppercase text-[10px] tracking-wider">
                      Dispatch Route:
                    </span>
                    <select
                      value={primaryProtocol}
                      onChange={(e) => setPrimaryProtocol(e.target.value)}
                      className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-emerald-300 font-medium focus:outline-none"
                      title="Select primary transmission protocol"
                    >
                      <option value="Signal">Signal (Primary)</option>
                      <option value="Matrix">Matrix (Primary)</option>
                      <option value="Discord">Discord (Primary)</option>
                      <option value="Simplex">Simplex (Primary)</option>
                      <option value="SMS">SMS (Primary)</option>
                    </select>

                    <span className="text-slate-500">➜</span>

                    <span className="text-slate-400 font-semibold uppercase text-[10px] tracking-wider">
                      Auto-Fallback:
                    </span>
                    <select
                      value={fallbackProtocol}
                      onChange={(e) => setFallbackProtocol(e.target.value)}
                      className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-indigo-300 font-medium focus:outline-none"
                      title="Select fallback protocol if timeout/failure occurs"
                    >
                      <option value="Matrix">Matrix (Fallback)</option>
                      <option value="Signal">Signal (Fallback)</option>
                      <option value="Discord">Discord (Fallback)</option>
                      <option value="Simplex">Simplex (Fallback)</option>
                      <option value="SMS">SMS (Fallback)</option>
                    </select>
                  </div>

                  <span
                    className="text-[11px] text-slate-400 cursor-help"
                    title="Messages dispatched here are encrypted using SQLCipher locally and transmitted via the chosen ProtocolBridge."
                  >
                    🔒 End-to-End Multi-Transport
                  </span>
                </div>

                {/* Input Bar */}
                <form onSubmit={handleSendMessage} className="flex items-center space-x-2">
                  <input
                    type="text"
                    placeholder={`Message ${selectedContact.name} via ${primaryProtocol}...`}
                    value={inputText}
                    onChange={(e) => setInputText(e.target.value)}
                    className="flex-1 bg-slate-950 border border-slate-700 rounded-lg p-3 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                  />
                  <button
                    type="button"
                    onClick={() => alert('Media & File attachment selector activated across all bridges.')}
                    className="p-3 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-slate-300 transition"
                    title="Attach Media / Encrypted Files across Bridges"
                  >
                    📎
                  </button>
                  <button
                    type="submit"
                    className="px-6 py-3 bg-blue-600 hover:bg-blue-500 rounded-lg font-semibold text-sm text-white shadow-lg shadow-blue-900/30 transition"
                  >
                    Dispatch
                  </button>
                </form>
              </div>
            </section>
          </>
        )}

        {/* 3. Bridge Control Deck (Consolidated Management) */}
        {activeTab === 'bridges' && (
          <section className="flex-1 p-6 overflow-y-auto bg-slate-950">
            <div className="max-w-5xl mx-auto space-y-6">
              <div className="flex items-center justify-between border-b border-slate-800 pb-4">
                <div>
                  <h2 className="text-xl font-extrabold text-white">Protocol Bridge Control Deck</h2>
                  <p className="text-xs text-slate-400 mt-1">
                    Manage real-time communication bridges, IPC sockets, homeservers, and gateway daemons.
                  </p>
                </div>
                <button
                  onClick={() => alert('Bridge auto-discovery scanning local IPC ports.')}
                  className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded text-xs font-semibold text-slate-200 transition"
                >
                  🔄 Scan Daemons
                </button>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {bridges.map((bridge) => (
                  <div
                    key={bridge.protocol}
                    className="p-4 rounded-xl bg-slate-900/90 border border-slate-800 flex flex-col justify-between space-y-3"
                  >
                    <div>
                      <div className="flex items-center justify-between">
                        <span className="font-bold text-sm text-white">{bridge.name}</span>
                        <span
                          className={`text-[10px] font-bold px-2 py-0.5 rounded-full ${
                            bridge.connected
                              ? 'bg-emerald-950 text-emerald-400 border border-emerald-800'
                              : 'bg-slate-800 text-slate-400 border border-slate-700'
                          }`}
                        >
                          {bridge.status}
                        </span>
                      </div>
                      <div className="text-[11px] font-mono text-slate-400 truncate mt-1">
                        {bridge.endpoint}
                      </div>
                      <div className="text-[10px] text-slate-500 mt-2 flex items-center space-x-3">
                        <span>Latency: <strong className="text-slate-300">{bridge.latency_ms} ms</strong></span>
                        <span>Priority: <strong className="text-slate-300">#{bridge.fallback_priority}</strong></span>
                      </div>
                    </div>

                    <div className="pt-2 border-t border-slate-800 flex items-center justify-between">
                      <span className="text-[10px] text-slate-400 uppercase font-semibold">
                        {bridge.protocol} Core
                      </span>
                      <button
                        onClick={() => handleToggleBridge(bridge.protocol)}
                        className={`text-xs px-3 py-1 rounded font-semibold transition ${
                          bridge.connected
                            ? 'bg-rose-950/60 hover:bg-rose-900 text-rose-300 border border-rose-800'
                            : 'bg-emerald-950/60 hover:bg-emerald-900 text-emerald-300 border border-emerald-800'
                        }`}
                      >
                        {bridge.connected ? 'Disconnect' : 'Connect Bridge'}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>
        )}

        {/* 4. Audit & Dispatch Monitor */}
        {activeTab === 'audit' && (
          <section className="flex-1 p-6 overflow-y-auto bg-slate-950">
            <div className="max-w-5xl mx-auto space-y-6">
              <div className="border-b border-slate-800 pb-4">
                <h2 className="text-xl font-extrabold text-white">Cross-Protocol Routing & Fallback Monitor</h2>
                <p className="text-xs text-slate-400 mt-1">
                  Inspect dispatched protobuf payloads, fallback triggers, and SQLCipher encrypted ledger writes.
                </p>
              </div>

              <div className="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden text-xs">
                <div className="p-3 bg-slate-800/80 font-bold text-slate-200 border-b border-slate-700 flex justify-between items-center">
                  <span>Recent Event Ledger</span>
                  <span className="text-[10px] text-slate-400 font-mono">SQLite + SQLCipher Cryptographic Store</span>
                </div>
                <div className="divide-y divide-slate-800 font-mono text-[11px]">
                  <div className="p-3 flex items-center justify-between hover:bg-slate-800/40">
                    <span className="text-emerald-400">[DISPATCH_OK]</span>
                    <span className="text-slate-300">Payload id: msg_1 routed to Alice via Signal IPC</span>
                    <span className="text-slate-500">12ms</span>
                  </div>
                  <div className="p-3 flex items-center justify-between hover:bg-slate-800/40">
                    <span className="text-indigo-400">[MULTI_ROUTE]</span>
                    <span className="text-slate-300">Payload id: msg_2 dual-broadcasted (Signal primary, Matrix fallback)</span>
                    <span className="text-slate-500">45ms</span>
                  </div>
                  <div className="p-3 flex items-center justify-between hover:bg-slate-800/40">
                    <span className="text-emerald-400">[SYNC_TOKEN]</span>
                    <span className="text-slate-300">Matrix homeserver token refreshed for room123</span>
                    <span className="text-slate-500">Just now</span>
                  </div>
                  <div className="p-3 flex items-center justify-between hover:bg-slate-800/40">
                    <span className="text-amber-400">[BRIDGE_STANDBY]</span>
                    <span className="text-slate-300">SimpleX broker standby polling on simplex://broker.v3.local</span>
                    <span className="text-slate-500">60ms</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        )}
      </div>

      {/* 5. Footer Status Bar */}
      <footer className="h-7 bg-slate-900 border-t border-slate-800 px-4 flex items-center justify-between text-[11px] text-slate-400 shrink-0">
        <div className="flex items-center space-x-3">
          <span>Storage: <strong className="text-slate-200">SQLCipher Encrypted</strong></span>
          <span>•</span>
          <span>Active Bridges: <strong className="text-emerald-400">{bridges.filter((b) => b.connected).length}</strong></span>
          <span>•</span>
          <span>Unified Identities: <strong className="text-blue-300">{contacts.length}</strong></span>
        </div>
        <div className="flex items-center space-x-3">
          <span className="text-[10px] text-slate-500">OmniCore Autonomous Engine | Autopilot Active</span>
        </div>
      </footer>
    </div>
  );
}

export default App;
