import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/prefs.dart';
import '../services/rradar_api.dart';
import 'about_screen.dart';
import 'capture_placeholder.dart';
import 'ledger_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.prefs});

  final AppPrefs prefs;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  String _status = '…';
  RradarCapabilities? _caps;
  int _count = 0;

  @override
  void initState() {
    super.initState();
    _loadCore();
  }

  Future<void> _loadCore() async {
    try {
      final ver = await rradarApi.apiVersion();
      final caps = await rradarApi.capabilities();
      final path = await rradarApi.defaultLedgerPath();
      await rradarApi.ensureLedger(path);
      final n = await rradarApi.count(path);
      if (!mounted) return;
      setState(() {
        _caps = caps;
        _count = n;
        _status =
            '$ver · schema ${caps.ledgerSchema} · txs $n · cloud=${caps.cloudSync}';
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _status = 'core: $e');
    }
  }

  Future<void> _simulateCapture() async {
    final path = await rradarApi.defaultLedgerPath();
    await rradarApi.processPath(path: 'mock://capture/receipt.jpg');
    await _loadCore();
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Mock capture confirmed (API)')),
    );
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
            tooltip: s.ledgerTitle,
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => LedgerScreen(strings: s),
                ),
              );
            },
            icon: const Icon(Icons.list_alt),
          ),
          IconButton(
            tooltip: s.aboutTitle,
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => AboutScreen(strings: s),
                ),
              );
            },
            icon: const Icon(Icons.info_outline),
          ),
        ],
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(
                _count == 0 ? s.emptyLedger : '${s.ledgerTitle}: $_count',
                textAlign: TextAlign.center,
              ),
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
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: _simulateCapture,
                icon: const Icon(Icons.science_outlined),
                label: const Text('Mock process (API)'),
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
