import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

/// Ledger list with optional tag filter (mock until FRB).
class LedgerScreen extends StatefulWidget {
  const LedgerScreen({super.key, required this.strings});

  final Strings strings;

  @override
  State<LedgerScreen> createState() => _LedgerScreenState();
}

class _LedgerScreenState extends State<LedgerScreen> {
  String _raw = '[]';
  String _tags = '[]';
  int _count = 0;
  String? _error;
  String? _tagFilter;
  final _tagCtrl = TextEditingController();

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _tagCtrl.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final path = await rradarApi.defaultLedgerPath();
      await rradarApi.ensureLedger(path);
      final n = await rradarApi.count(path);
      final list = await rradarApi.queryJson(
        path,
        limit: 50,
        tag: _tagFilter,
      );
      final tags = await rradarApi.listTagsJson(path);
      if (!mounted) return;
      setState(() {
        _count = n;
        _raw = list;
        _tags = tags;
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
            : ListView(
                children: [
                  Text(
                    '$_count transactions · filter tag: ${_tagFilter ?? "(all)"}',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 8),
                  Text('tags: $_tags',
                      style: Theme.of(context)
                          .textTheme
                          .bodySmall
                          ?.copyWith(fontFamily: 'monospace')),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _tagCtrl,
                          decoration: const InputDecoration(
                            labelText: 'Tag filter',
                            hintText: 'demo',
                            isDense: true,
                          ),
                          onSubmitted: (v) {
                            setState(() {
                              _tagFilter = v.trim().isEmpty ? null : v.trim();
                            });
                            _load();
                          },
                        ),
                      ),
                      const SizedBox(width: 8),
                      FilledButton.tonal(
                        onPressed: () {
                          setState(() {
                            _tagFilter = _tagCtrl.text.trim().isEmpty
                                ? null
                                : _tagCtrl.text.trim();
                          });
                          _load();
                        },
                        child: const Text('Filter'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  if (_count == 0)
                    Text(s.emptyLedger, textAlign: TextAlign.center)
                  else
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
