import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [status, setStatus] = useState('Initializing...');

  useEffect(() => {
    // Check if running within Tauri context
    const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI_IPC__' in window;
    if (isTauri) {
      invoke('init_engine')
        .then((res) => setStatus(res as string))
        .catch((err) => setStatus(`Error: ${err}`));
    } else {
      console.log('Not running within Tauri, mocking engine init.');
      setStatus('Engine and bridges initialized successfully (Mock)');
    }
  }, []);

  return (
    <div className="flex h-screen w-screen bg-gray-900 text-white font-sans overflow-hidden">
      {/* Universal Sidebar & System Tray Proxy */}
      <div className="w-1/4 border-r border-gray-700 flex flex-col bg-gray-800">
        <div className="p-4 border-b border-gray-700 flex justify-between items-center">
          <span className="font-bold text-lg">OmniCore</span>
          <div className="relative group">
            <button className="text-gray-400 hover:text-white" aria-label="System Tray Menu">
              ⚙️
            </button>
            <div className="absolute right-0 mt-2 w-48 bg-gray-700 rounded-md shadow-lg py-1 z-10 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
              <a href="#" className="block px-4 py-2 text-sm text-gray-200 hover:bg-gray-600">Global Settings</a>
              <a href="#" className="block px-4 py-2 text-sm text-gray-200 hover:bg-gray-600">Manage Bridges</a>
              <div className="border-t border-gray-600 my-1"></div>
              <a href="#" className="block px-4 py-2 text-sm text-red-400 hover:bg-gray-600" onClick={() => console.log('Quit requested')}>Quit Server</a>
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          <h2 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Unified Contacts</h2>
          <div className="space-y-1">
            {/* Mock Contacts */}
            {['Alice (Signal)', 'Bob (Matrix)', 'Charlie (Discord)'].map(
              (contact) => (
                <div
                  key={contact}
                  className="p-3 rounded bg-gray-700 hover:bg-gray-600 cursor-pointer transition-colors flex justify-between items-center"
                >
                  <span>{contact}</span>
                  <span className="w-2 h-2 rounded-full bg-green-500"></span>
                </div>
              )
            )}
          </div>
        </div>
      </div>

      {/* Main Chat Feed */}
      <div className="flex-1 flex flex-col bg-gray-900">
        <div className="p-4 border-b border-gray-700 font-semibold flex justify-between items-center">
          <span>Unified Chat Feed</span>
          <span className="text-xs text-green-400 bg-gray-800 px-2 py-1 rounded">
            {status}
          </span>
        </div>
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {/* Mock Messages */}
          <div className="flex flex-col space-y-1">
            <span className="text-xs text-gray-400 font-bold">Alice (Signal)</span>
            <div className="bg-gray-700 p-3 rounded-lg w-max max-w-xl self-start">
              Hey, is this the new OmniCore client?
            </div>
          </div>
          <div className="flex flex-col space-y-1 items-end">
            <span className="text-xs text-gray-400 font-bold">You (Multi-Send)</span>
            <div className="bg-blue-600 p-3 rounded-lg w-max max-w-xl self-end">
              Yes! Routing works across Matrix and Signal right now.
            </div>
          </div>
        </div>

        {/* Multi-Platform Message Composition Box */}
        <div className="p-4 border-t border-gray-700 bg-gray-800 flex flex-col space-y-2 relative group">
          <div className="flex space-x-2 text-xs text-gray-400 font-semibold uppercase px-1">
            <span>Routing via:</span>
            <span className="text-green-400 bg-gray-900 px-2 py-0.5 rounded cursor-pointer hover:bg-gray-700 transition" title="Primary route for Alice">Signal</span>
            <span className="text-blue-400 bg-gray-900 px-2 py-0.5 rounded cursor-pointer hover:bg-gray-700 transition" title="Fallback route for Alice">Matrix</span>
            <span className="text-purple-400 bg-gray-900 px-2 py-0.5 rounded cursor-pointer hover:bg-gray-700 transition" title="Broadcast to all linked accounts">Discord</span>
          </div>

          <div className="absolute -top-12 left-4 bg-gray-700 text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
            Messages typed here can be broadcasted or smartly routed across all linked platforms.
          </div>
          <form
            className="flex items-center space-x-2"
            onSubmit={(e) => e.preventDefault()}
          >
            <input
              type="text"
              placeholder="Type a message (Markdown supported)..."
              className="flex-1 bg-gray-900 text-white rounded p-3 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
            <div className="flex space-x-2">
              <button
                type="button"
                className="bg-gray-700 hover:bg-gray-600 px-4 py-3 rounded transition-colors"
                title="Attach Media / Files across unified protocols"
              >
                📎
              </button>
              <button
                type="submit"
                className="bg-blue-600 hover:bg-blue-500 px-6 py-3 rounded font-semibold transition-colors"
              >
                Send
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

export default App;
