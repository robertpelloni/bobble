import React, { useState } from 'react';
import {
  View,
  Text,
  FlatList,
  TextInput,
  TouchableOpacity,
  StyleSheet,
  SafeAreaView,
} from 'react-native';
import { ProtocolBadge } from '../components/ProtocolBadge';

interface Message {
  id: string;
  sender: string;
  content: string;
  protocol: string;
  timestamp: string;
  isOutgoing: boolean;
}

export const ConversationScreen: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: 'm1',
      sender: 'Alice Montgomery',
      content: 'OmniCore mobile client connected to local engine!',
      protocol: 'Signal',
      timestamp: '11:05 AM',
      isOutgoing: false,
    },
    {
      id: 'm2',
      sender: 'You',
      content: 'Cross-platform unified routing operational across iOS and Android.',
      protocol: 'Matrix',
      timestamp: '11:06 AM',
      isOutgoing: true,
    },
  ]);
  const [input, setInput] = useState('');
  const [primaryProto, setPrimaryProto] = useState('Signal');

  const handleSend = () => {
    if (!input.trim()) return;
    const newMsg: Message = {
      id: `m_${Date.now()}`,
      sender: 'You',
      content: input,
      protocol: primaryProto,
      timestamp: 'Just now',
      isOutgoing: true,
    };
    setMessages((prev) => [...prev, newMsg]);
    setInput('');
  };

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.headerTitle}>Alice Montgomery</Text>
        <Text style={styles.headerSubtitle}>Unified ID: alice_unified_1</Text>
        <View style={styles.badgeRow}>
          <ProtocolBadge protocol={primaryProto} />
          <ProtocolBadge protocol="Matrix" isFallback />
        </View>
      </View>

      <FlatList
        data={messages}
        keyExtractor={(item) => item.id}
        contentContainerStyle={styles.messageList}
        renderItem={({ item }) => (
          <View
            style={[
              styles.bubble,
              item.isOutgoing ? styles.outgoingBubble : styles.incomingBubble,
            ]}
          >
            <View style={styles.bubbleHeader}>
              <Text style={styles.senderText}>{item.sender}</Text>
              <Text style={styles.timeText}>{item.timestamp}</Text>
            </View>
            <Text style={styles.contentText}>{item.content}</Text>
            <Text style={styles.protocolFooter}>Transport: {item.protocol}</Text>
          </View>
        )}
      />

      <View style={styles.inputBar}>
        <TextInput
          style={styles.textInput}
          placeholder={`Message via ${primaryProto}...`}
          placeholderTextColor="#64748b"
          value={input}
          onChangeText={setInput}
        />
        <TouchableOpacity style={styles.sendButton} onPress={handleSend}>
          <Text style={styles.sendButtonText}>Send</Text>
        </TouchableOpacity>
      </View>
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
    borderBottomWidth: 1,
    borderBottomColor: '#1e293b',
    backgroundColor: '#0f172a',
  },
  headerTitle: {
    color: '#ffffff',
    fontSize: 18,
    fontWeight: 'bold',
  },
  headerSubtitle: {
    color: '#94a3b8',
    fontSize: 12,
    marginTop: 2,
  },
  badgeRow: {
    flexDirection: 'row',
    marginTop: 8,
  },
  messageList: {
    padding: 16,
  },
  bubble: {
    padding: 12,
    borderRadius: 14,
    marginBottom: 12,
    maxWidth: '85%',
  },
  incomingBubble: {
    backgroundColor: '#1e293b',
    alignSelf: 'flex-start',
  },
  outgoingBubble: {
    backgroundColor: '#2563eb',
    alignSelf: 'flex-end',
  },
  bubbleHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 4,
  },
  senderText: {
    color: '#cbd5e1',
    fontSize: 11,
    fontWeight: 'bold',
  },
  timeText: {
    color: '#94a3b8',
    fontSize: 10,
  },
  contentText: {
    color: '#ffffff',
    fontSize: 14,
    lineHeight: 20,
  },
  protocolFooter: {
    color: '#93c5fd',
    fontSize: 10,
    marginTop: 6,
  },
  inputBar: {
    flexDirection: 'row',
    padding: 12,
    backgroundColor: '#0f172a',
    borderTopWidth: 1,
    borderTopColor: '#1e293b',
  },
  textInput: {
    flex: 1,
    backgroundColor: '#020617',
    color: '#ffffff',
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderWidth: 1,
    borderColor: '#334155',
  },
  sendButton: {
    backgroundColor: '#2563eb',
    borderRadius: 8,
    paddingHorizontal: 16,
    justifyContent: 'center',
    marginLeft: 8,
  },
  sendButtonText: {
    color: '#ffffff',
    fontWeight: 'bold',
  },
});
