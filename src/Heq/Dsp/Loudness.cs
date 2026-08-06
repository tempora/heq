using System;
using Heq.Model;
using Heq.Storage;

namespace Heq.Dsp
{
    public static class Loudness
    {
        private const double FreqMin = 20.0;
        private const double FreqMax = 20000.0;
        private const int Bins = 240;

        private const double WeightingRate = 48000.0;

        private static readonly double[] Freqs = new double[Bins];
        private static readonly double[] Weights = new double[Bins];
        private static readonly double WeightSum;

        static Loudness()
        {
            var shelf = new Biquad
            {
                B0 = 1.53512485958697,
                B1 = -2.69169618940638,
                B2 = 1.19839281085285,
                A0 = 1.0,
                A1 = -1.69065929318241,
                A2 = 0.73248077421585,
            };
            var highPass = new Biquad
            {
                B0 = 1.0,
                B1 = -2.0,
                B2 = 1.0,
                A0 = 1.0,
                A1 = -1.99004745483398,
                A2 = 0.99007225036621,
            };

            double logMin = Math.Log10(FreqMin);
            double span = Math.Log10(FreqMax) - logMin;

            double sum = 0;
            for (int i = 0; i < Bins; i++)
            {
                double f = Math.Pow(10, logMin + span * i / (Bins - 1.0));
                double w = 2.0 * Math.PI * f / WeightingRate;

                Freqs[i] = f;
                Weights[i] = Math.Pow(10, (shelf.GainDb(w) + highPass.GainDb(w)) / 10.0);
                sum += Weights[i];
            }
            WeightSum = sum;
        }

        public static double LevelDb(EqModel model)
        {
            if (model == null) return 0;

            bool split = model.HasPerChannelBands;
            double energy = 0;

            for (int i = 0; i < Bins; i++)
            {
                double f = Freqs[i];
                double gain;

                if (split)
                {
                    double l = Power(model.ResponseDb(f, ChannelTarget.Left));
                    double r = Power(model.ResponseDb(f, ChannelTarget.Right));
                    gain = (l + r) * 0.5;
                }
                else
                {
                    gain = Power(model.ResponseDb(f));
                }

                energy += Weights[i] * gain;
            }

            double mean = energy / WeightSum;
            double curve = mean <= 1e-12 ? -120.0 : 10.0 * Math.Log10(mean);
            return curve + model.BasePreampDb;
        }

        public static double LevelDb(Preset preset, double sampleRate)
        {
            if (preset == null) return 0;
            var m = new EqModel { SampleRate = sampleRate <= 0 ? 48000 : sampleRate };
            preset.ApplyTo(m);
            return LevelDb(m);
        }

        private static double Power(double db) => Math.Pow(10, db / 10.0);
    }
}
