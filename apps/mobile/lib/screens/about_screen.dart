import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

class AboutScreen extends StatefulWidget {
  const AboutScreen({super.key, required this.strings});

  final Strings strings;

  @override
  State<AboutScreen> createState() => _AboutScreenState();
}

class _AboutScreenState extends State<AboutScreen> {
  RradarCapabilities? _caps;
  String _version = '';
  String _paths = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final v = await rradarApi.apiVersion();
    final c = await rradarApi.capabilities();
    final ledger = await rradarApi.defaultLedgerPath();
    final inbox = await rradarApi.defaultInboxPath();
    final rules = await rradarApi.defaultRulesPath();
    if (!mounted) return;
    setState(() {
      _version = v;
      _caps = c;
      _paths = 'ledger: $ledger\ninbox: $inbox\nrules: $rules';
    });
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.strings;
    final c = _caps;
    return Scaffold(
      appBar: AppBar(title: Text(s.aboutTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(_version, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          if (c != null) ...[
            _row('schema', '${c.ledgerSchema}'),
            _row('cloud_sync', '${c.cloudSync}'),
            _row('official_relay', '${c.officialRelay}'),
            _row('handoff', '${c.multiDeviceHandoff}'),
            _row('rule_packs', '${c.rulePacks}'),
            _row('tags/attachments', '${c.tagsAttachments}'),
            _row('attachment_store', '${c.attachmentStore}'),
            _row('backup+blobs', '${c.backupIncludesAttachments}'),
            _row('capture_oneshot', '${c.captureOneshot}'),
            _row('local_http', '${c.localHttpServe}'),
            const SizedBox(height: 8),
            Text(c.notes, style: Theme.of(context).textTheme.bodySmall),
          ],
          const Divider(height: 32),
          Text(_paths, style: Theme.of(context).textTheme.bodySmall?.copyWith(fontFamily: 'monospace')),
          const SizedBox(height: 16),
          Text(s.ffiPending, style: Theme.of(context).textTheme.bodySmall),
          Text(s.privacyNote, style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
    );
  }

  Widget _row(String k, String v) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          SizedBox(width: 140, child: Text(k, style: const TextStyle(fontFamily: 'monospace'))),
          Expanded(child: Text(v)),
        ],
      ),
    );
  }
}
