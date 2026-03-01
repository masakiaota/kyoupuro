#!/usr/bin/env perl
use strict;
use warnings;

sub usage {
    die "Usage: $0 <CONST_NAME> <OUT_FILE> <LIB:LIMIT> [<LIB:LIMIT> ...]\n";
}

@ARGV >= 3 or usage();
my $const_name = shift @ARGV;
my $out_file = shift @ARGV;
my @specs = @ARGV;

my %seen;
my @all;

for my $spec (@specs) {
    my ($path, $lim) = split /:/, $spec, 2;
    defined $path && defined $lim or usage();
    $lim =~ /^\d+$/ or die "invalid limit in spec: $spec\n";
    my $limit = int($lim);
    next if $limit <= 0;

    open my $fh, '<', $path or die "cannot open $path: $!\n";
    my $m = undef;
    my @rules = ();
    my $taken = 0;

    my $flush = sub {
        return if !defined $m;
        return if scalar(@rules) != $m;
        my $key = join(';', map { join(',', @$_) } @rules);
        if (!$seen{$key}) {
            $seen{$key} = 1;
            push @all, {
                m => $m,
                rules => [ @rules ],
            };
            $taken++;
        }
        $m = undef;
        @rules = ();
    };

    while (my $line = <$fh>) {
        if ($line =~ /^\[candidate\b.*\bm=(\d+)\b/) {
            $flush->();
            last if $taken >= $limit;
            $m = int($1);
            @rules = ();
            next;
        }
        if (defined $m && $line =~ /^state\s+\d+:\s+([RLF])\s+(\d+)\s+([RLF])\s+(\d+)\s*$/) {
            push @rules, [ $1, int($2), $3, int($4) ];
            next;
        }
        if (defined $m && $line =~ /^\s*$/) {
            $flush->();
            last if $taken >= $limit;
            next;
        }
    }
    $flush->();
    close $fh;
}

sub act_expr {
    my ($c) = @_;
    return 'ACT_R' if $c eq 'R';
    return 'ACT_L' if $c eq 'L';
    return 'ACT_F' if $c eq 'F';
    die "unknown action char: $c\n";
}

open my $out, '>', $out_file or die "cannot write $out_file: $!\n";
print $out "const $const_name: &[TemplateDef] = &[\n";
for my $c (@all) {
    my $m = $c->{m};
    print $out "    TemplateDef {\n";
    print $out "        m: $m,\n";
    print $out "        rules: &[\n";
    for my $r (@{$c->{rules}}) {
        my ($a0, $b0, $a1, $b1) = @$r;
        printf $out "            (%s, %d, %s, %d),\n", act_expr($a0), $b0, act_expr($a1), $b1;
    }
    print $out "        ],\n";
    print $out "    },\n";
}
print $out "];\n";
close $out;

print STDERR "generated $out_file with " . scalar(@all) . " templates\n";
