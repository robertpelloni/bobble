import React from 'react';
import { View, Text, StyleSheet } from 'react-native';

interface ProtocolBadgeProps {
  protocol: string;
  isFallback?: boolean;
}

export const ProtocolBadge: React.FC<ProtocolBadgeProps> = ({ protocol, isFallback }) => {
  const getBgColor = () => {
    switch (protocol.toLowerCase()) {
      case 'signal':
        return '#0284c7';
      case 'matrix':
        return '#059669';
      case 'discord':
        return '#6366f1';
      case 'simplex':
        return '#7c3aed';
      case 'sms':
        return '#d97706';
      default:
        return '#475569';
    }
  };

  return (
    <View style={[styles.badge, { backgroundColor: getBgColor() }]}>
      <Text style={styles.text}>
        {protocol} {isFallback ? '(Fallback)' : ''}
      </Text>
    </View>
  );
};

const styles = StyleSheet.create({
  badge: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 6,
    alignSelf: 'flex-start',
    marginRight: 6,
  },
  text: {
    color: '#ffffff',
    fontSize: 11,
    fontWeight: 'bold',
  },
});
