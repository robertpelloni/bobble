import React, { useState } from 'react';
import { View, Text, TouchableOpacity, StyleSheet, StatusBar } from 'react-native';
import { ConversationScreen } from './src/screens/ConversationScreen';
import { BridgeDeckScreen } from './src/screens/BridgeDeckScreen';
import { SettingsScreen } from './src/screens/SettingsScreen';

export default function App() {
  const [currentTab, setCurrentTab] = useState<'chat' | 'bridges' | 'settings'>('chat');

  return (
    <View style={styles.appContainer}>
      <StatusBar barStyle="light-content" backgroundColor="#020617" />
      <View style={styles.contentContainer}>
        {currentTab === 'chat' && <ConversationScreen />}
        {currentTab === 'bridges' && <BridgeDeckScreen />}
        {currentTab === 'settings' && <SettingsScreen />}
      </View>
      <View style={styles.bottomNav}>
        <TouchableOpacity
          style={[styles.navButton, currentTab === 'chat' && styles.navButtonActive]}
          onPress={() => setCurrentTab('chat')}
        >
          <Text style={[styles.navText, currentTab === 'chat' && styles.navTextActive]}>
            💬 Chat
          </Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.navButton, currentTab === 'bridges' && styles.navButtonActive]}
          onPress={() => setCurrentTab('bridges')}
        >
          <Text style={[styles.navText, currentTab === 'bridges' && styles.navTextActive]}>
            🔌 Bridges
          </Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.navButton, currentTab === 'settings' && styles.navButtonActive]}
          onPress={() => setCurrentTab('settings')}
        >
          <Text style={[styles.navText, currentTab === 'settings' && styles.navTextActive]}>
            ⚙️ Settings
          </Text>
        </TouchableOpacity>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  appContainer: {
    flex: 1,
    backgroundColor: '#020617',
  },
  contentContainer: {
    flex: 1,
  },
  bottomNav: {
    flexDirection: 'row',
    height: 60,
    backgroundColor: '#0f172a',
    borderTopWidth: 1,
    borderTopColor: '#1e293b',
  },
  navButton: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  navButtonActive: {
    borderTopWidth: 2,
    borderTopColor: '#3b82f6',
  },
  navText: {
    color: '#64748b',
    fontWeight: '600',
    fontSize: 13,
  },
  navTextActive: {
    color: '#3b82f6',
  },
});
