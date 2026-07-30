import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/prefs.dart';
import '../services/rradar_api.dart';
import 'capture_placeholder.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.prefs});

  final AppPrefs prefs;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  String _status = '…';
  RradarCapabilities? _caps;

  @override
  void initState() {
    super.initState();
    _loadCore();
  }

  Future<void> _loadCore() async {
    try {
      final ver = await rradarApi.apiVersion();
      final caps = await rradarApi.capabilities();
      final n = await rradarApi.count(await rradarApi.defaultLedgerPath());
      if (!mounted) return;
      setState(() {
        _caps = caps;
        _status =
            '$ver · schema ${caps.ledgerSchema} · txs $n · cloud=${caps.cloudSync}';
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _status = 'core: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = Strings.of(widget.prefs.locale);
    final caps = _caps;
    return Scaffold(
      appBar: AppBar(
        title: Text(s.homeTitle),
        actions: [
          IconButton(
            tooltip: widget.prefs.locale == 'zh-TW' ? 'EN' : '中文',
            onPressed: () {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text(s.ffiPending)),
              );
            },
            icon: const Icon(Icons.language),
          ),
        ],
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(s.emptyLedger, textAlign: TextAlign.center),
              const SizedBox(height: 12),
              Text(
                s.privacyNote,
                style: Theme.of(context).textTheme.bodySmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 16),
              Text(
                _status,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.primary,
                      fontFamily: 'monospace',
                    ),
                textAlign: TextAlign.center,
              ),
              if (caps != null) ...[
                const SizedBox(height: 8),
                Text(
                  caps.notes,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.outline,
                      ),
                  textAlign: TextAlign.center,
                ),
              ],
              const SizedBox(height: 8),
              Text(
                s.ffiPending,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.outline,
                    ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => CapturePlaceholder(strings: s),
            ),
          );
        },
        icon: const Icon(Icons.photo_camera),
        label: Text(s.captureCta),
      ),
    );
  }
}
