import React, { useState } from 'react';
import { daemon } from '../services/daemon';
import {
  View,
  Text,
  TextInput,
  TouchableOpacity,
  Switch,
  StyleSheet,
  SafeAreaView,
  ScrollView,
} from 'react-native';

export const SettingsScreen: React.FC = () => {
  const [daemonEndpoint, setDaemonEndpointState] = useState(daemon.getEndpoint());
  const setDaemonEndpoint = (url: string) => {
    setDaemonEndpointState(url);
    daemon.setEndpoint(url);
  };
  const [torEnabled, setTorEnabled] = useState(true);
  const [onionAddress, setOnionAddress] = useState(
    'omnicore3x7abc1234567890abcdefghijklmnopqrstuvwxyz234567.onion'
  );
  const [identityFingerprint, setIdentityFingerprint] = useState(
    'E8B2:3F1A:99C0:D44E:71B8:502C:129A:FE88'
  );
  const [backupSync, setBackupSync] = useState(true);

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.title}>Mobile Engine Settings</Text>
        <Text style={styles.subtitle}>Configure local daemon, Tor routing, and cryptographic identity</Text>
      </View>

      <ScrollView contentContainerStyle={styles.scrollContent}>
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Daemon Connection</Text>
          <Text style={styles.label}>OmniCore Rust Daemon Endpoint</Text>
          <TextInput
            style={styles.input}
            value={daemonEndpoint}
            onChangeText={setDaemonEndpoint}
            placeholder="ws://127.0.0.1:8765"
            placeholderTextColor="#64748b"
          />
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Privacy & Tor Routing</Text>
          <View style={styles.row}>
            <View style={styles.rowText}>
              <Text style={styles.rowLabel}>Route through Tor SOCKS5</Text>
              <Text style={styles.rowDesc}>Prevents ISP and carrier traffic correlation</Text>
            </View>
            <Switch
              value={torEnabled}
              onValueChange={setTorEnabled}
              trackColor={{ false: '#334155', true: '#2563eb' }}
            />
          </View>
          <Text style={[styles.label, { marginTop: 12 }]}>Node .onion Address</Text>
          <TextInput
            style={[styles.input, styles.monoInput]}
            value={onionAddress}
            onChangeText={setOnionAddress}
            editable={false}
          />
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Cryptographic Identity</Text>
          <Text style={styles.label}>Ed25519 Root Fingerprint</Text>
          <TextInput
            style={[styles.input, styles.monoInput]}
            value={identityFingerprint}
            editable={false}
          />
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionButtonText}>Export Identity Card (JSON)</Text>
          </TouchableOpacity>
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Zero-Knowledge Backup</Text>
          <View style={styles.row}>
            <View style={styles.rowText}>
              <Text style={styles.rowLabel}>Encrypted Vault Sync</Text>
              <Text style={styles.rowDesc}>Automated chunked backup sync with SHA-256 verification</Text>
            </View>
            <Switch
              value={backupSync}
              onValueChange={setBackupSync}
              trackColor={{ false: '#334155', true: '#2563eb' }}
            />
          </View>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#020617',
  },
  header: {
    padding: 16,
    backgroundColor: '#0f172a',
    borderBottomWidth: 1,
    borderBottomColor: '#1e293b',
  },
  title: {
    color: '#ffffff',
    fontSize: 18,
    fontWeight: 'bold',
  },
  subtitle: {
    color: '#94a3b8',
    fontSize: 12,
    marginTop: 2,
  },
  scrollContent: {
    padding: 16,
  },
  section: {
    backgroundColor: '#0f172a',
    padding: 16,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: '#1e293b',
    marginBottom: 16,
  },
  sectionTitle: {
    color: '#60a5fa',
    fontSize: 14,
    fontWeight: 'bold',
    marginBottom: 12,
    textTransform: 'uppercase',
  },
  label: {
    color: '#cbd5e1',
    fontSize: 12,
    marginBottom: 6,
    fontWeight: '500',
  },
  input: {
    backgroundColor: '#020617',
    borderWidth: 1,
    borderColor: '#334155',
    borderRadius: 8,
    color: '#ffffff',
    paddingHorizontal: 12,
    paddingVertical: 10,
    fontSize: 13,
  },
  monoInput: {
    fontFamily: 'monospace',
    color: '#94a3b8',
    fontSize: 11,
  },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  rowText: {
    flex: 1,
    paddingRight: 12,
  },
  rowLabel: {
    color: '#ffffff',
    fontSize: 14,
    fontWeight: '600',
  },
  rowDesc: {
    color: '#64748b',
    fontSize: 11,
    marginTop: 2,
  },
  actionButton: {
    backgroundColor: '#1e293b',
    borderWidth: 1,
    borderColor: '#3b82f6',
    borderRadius: 8,
    paddingVertical: 10,
    alignItems: 'center',
    marginTop: 12,
  },
  actionButtonText: {
    color: '#60a5fa',
    fontWeight: '600',
    fontSize: 13,
  },
});
