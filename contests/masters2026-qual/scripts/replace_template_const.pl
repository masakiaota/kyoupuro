#!/usr/bin/env perl
use strict;
use warnings;

@ARGV == 3 or die "Usage: $0 <TARGET_RS> <CONST_NAME> <FRAGMENT_FILE>\n";
my ($target, $const_name, $frag_file) = @ARGV;

open my $fh, '<', $target or die "cannot open $target: $!\n";
local $/;
my $src = <$fh>;
close $fh;

open my $ff, '<', $frag_file or die "cannot open $frag_file: $!\n";
my $frag = <$ff>;
close $ff;

my $pat = qr/const \Q$const_name\E: &\[TemplateDef\] = &\[(?s:.*?)\n\];/m;
$src =~ s/$pat/$frag/s or die "failed to replace constant $const_name in $target\n";

open my $out, '>', $target or die "cannot write $target: $!\n";
print $out $src;
close $out;

print STDERR "replaced $const_name in $target\n";
