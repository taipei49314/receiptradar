import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

/// Simple ledger list driven by [RradarApi] (mock until FRB).
class LedgerScreen extends StatefulWidget {
  const LedgerScreen({super.key, required this.strings});

  final Strings strings;

  @override
  State<LedgerScreen> createState() => _LedgerScreenState();
}

class _LedgerScreenState extends State<LedgerScreen> {
  String _raw = '[]';
  int _count = 0;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final path = await rradarApi.defaultLedgerPath();
      await rradarApi.ensureLedger(path);
      final n = await rradarApi.count(path);
      final list = await rradarApi.listJson(path, limit: 50);
      if (!mounted) return;
      setState(() {
        _count = n;
        _raw = list;
        _error = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.strings;
    return Scaffold(
      appBar: AppBar(
        title: Text(s.ledgerTitle),
        actions: [
          IconButton(
            tooltip: s.refresh,
            onPressed: _load,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: _error != null
            ? Center(child: Text(_error!))
            : _count == 0
                ? Center(child: Text(s.emptyLedger, textAlign: TextAlign.center))
                : ListView(
                    children: [
                      Text(
                        '$_count transactions (mock API until FRB)',
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                      const SizedBox(height: 12),
                      SelectableText(
                        _raw,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              fontFamily: 'monospace',
                            ),
                      ),
                    ],
                  ),
      ),
    );
  }
}
