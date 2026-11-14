import React from 'react';
import { View, Text, FlatList, StyleSheet, SafeAreaView } from 'react-native';

interface BridgeItem {
  protocol: string;
  endpoint: string;
  status: string;
  latencyMs: number;
}

const BRIDGES: BridgeItem[] = [
  { protocol: 'Signal', endpoint: 'unix:///var/run/signald.sock', status: 'Active', latencyMs: 12 },
  { protocol: 'Matrix', endpoint: 'https://matrix.org', status: 'Active', latencyMs: 45 },
  { protocol: 'Discord', endpoint: 'wss://gateway.discord.gg', status: 'Active', latencyMs: 38 },
  { protocol: 'SimpleX', endpoint: 'smp://simplex.broker.local:5223', status: 'Standby', latencyMs: 60 },
  { protocol: 'Tox', endpoint: 'nodes.tox.chat:33445', status: 'Standby', latencyMs: 95 },
  { protocol: 'SMS', endpoint: '/dev/ttyUSB0', status: 'Ready', latencyMs: 110 },
  { protocol: 'iMessage', endpoint: 'https://relay.imessage.local', status: 'Configured', latencyMs: 50 },
  { protocol: 'Nostr', endpoint: 'wss://relay.damus.io', status: 'Standby', latencyMs: 80 },
];

export const BridgeDeckScreen: React.FC = () => {
  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.title}>Mobile Bridge Telemetry</Text>
        <Text style={styles.subtitle}>Direct connection to OmniCore Rust daemon</Text>
      </View>
      <FlatList
        data={BRIDGES}
        keyExtractor={(item) => item.protocol}
        renderItem={({ item }) => (
          <View style={styles.card}>
            <View style={styles.cardRow}>
              <Text style={styles.protocolName}>{item.protocol}</Text>
              <Text
                style={[
                  styles.statusBadge,
                  item.status === 'Active' ? styles.statusActive : styles.statusStandby,
                ]}
              >
                {item.status}
              </Text>
            </View>
            <Text style={styles.endpoint}>{item.endpoint}</Text>
            <Text style={styles.latency}>Latency: {item.latencyMs} ms</Text>
          </View>
        )}
      />
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
  card: {
    backgroundColor: '#0f172a',
    marginHorizontal: 16,
    marginTop: 12,
    padding: 14,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: '#1e293b',
  },
  cardRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  protocolName: {
    color: '#ffffff',
    fontSize: 15,
    fontWeight: 'bold',
  },
  statusBadge: {
    fontSize: 11,
    fontWeight: 'bold',
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 6,
  },
  statusActive: {
    backgroundColor: '#064e3b',
    color: '#34d399',
  },
  statusStandby: {
    backgroundColor: '#1e293b',
    color: '#94a3b8',
  },
  endpoint: {
    color: '#94a3b8',
    fontSize: 12,
    fontFamily: 'monospace',
    marginTop: 6,
  },
  latency: {
    color: '#64748b',
    fontSize: 11,
    marginTop: 4,
  },
});
