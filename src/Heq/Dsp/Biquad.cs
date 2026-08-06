using System;

namespace Heq.Dsp
{
    public struct Biquad
    {
        public double B0, B1, B2, A0, A1, A2;

        public double MagnitudeSquared(double w)
        {
            double s = Math.Sin(w * 0.5);
            double phi = s * s;

            double bSum = B0 + B1 + B2;
            double aSum = A0 + A1 + A2;

            double num = bSum * bSum
                         - 4.0 * (B0 * B1 + 4.0 * B0 * B2 + B1 * B2) * phi
                         + 16.0 * B0 * B2 * phi * phi;
            double den = aSum * aSum
                         - 4.0 * (A0 * A1 + 4.0 * A0 * A2 + A1 * A2) * phi
                         + 16.0 * A0 * A2 * phi * phi;

            if (den <= 0) return 0;
            if (num < 0) num = 0;
            return num / den;
        }

        public double GainDb(double w)
        {
            double m2 = MagnitudeSquared(w);
            return m2 <= 1e-20 ? -200.0 : 10.0 * Math.Log10(m2);
        }

        public static Biquad Peaking(double freq, double sampleRate, double gainDb, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double a = Math.Pow(10.0, gainDb / 40.0);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = 1 + alpha * a,
                B1 = -2 * cos,
                B2 = 1 - alpha * a,
                A0 = 1 + alpha / a,
                A1 = -2 * cos,
                A2 = 1 - alpha / a,
            };
        }

        public static Biquad LowShelf(double freq, double sampleRate, double gainDb, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double a = Math.Pow(10.0, gainDb / 40.0);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);
            double sqrtA2Alpha = 2.0 * Math.Sqrt(a) * alpha;

            return new Biquad
            {
                B0 = a * ((a + 1) - (a - 1) * cos + sqrtA2Alpha),
                B1 = 2 * a * ((a - 1) - (a + 1) * cos),
                B2 = a * ((a + 1) - (a - 1) * cos - sqrtA2Alpha),
                A0 = (a + 1) + (a - 1) * cos + sqrtA2Alpha,
                A1 = -2 * ((a - 1) + (a + 1) * cos),
                A2 = (a + 1) + (a - 1) * cos - sqrtA2Alpha,
            };
        }

        public static Biquad HighShelf(double freq, double sampleRate, double gainDb, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double a = Math.Pow(10.0, gainDb / 40.0);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);
            double sqrtA2Alpha = 2.0 * Math.Sqrt(a) * alpha;

            return new Biquad
            {
                B0 = a * ((a + 1) + (a - 1) * cos + sqrtA2Alpha),
                B1 = -2 * a * ((a - 1) + (a + 1) * cos),
                B2 = a * ((a + 1) + (a - 1) * cos - sqrtA2Alpha),
                A0 = (a + 1) - (a - 1) * cos + sqrtA2Alpha,
                A1 = 2 * ((a - 1) - (a + 1) * cos),
                A2 = (a + 1) - (a - 1) * cos - sqrtA2Alpha,
            };
        }

        public static Biquad HighPass(double freq, double sampleRate, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = (1 + cos) / 2,
                B1 = -(1 + cos),
                B2 = (1 + cos) / 2,
                A0 = 1 + alpha,
                A1 = -2 * cos,
                A2 = 1 - alpha,
            };
        }

        public static Biquad LowPass(double freq, double sampleRate, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = (1 - cos) / 2,
                B1 = 1 - cos,
                B2 = (1 - cos) / 2,
                A0 = 1 + alpha,
                A1 = -2 * cos,
                A2 = 1 - alpha,
            };
        }

        public static Biquad Notch(double freq, double sampleRate, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = 1,
                B1 = -2 * cos,
                B2 = 1,
                A0 = 1 + alpha,
                A1 = -2 * cos,
                A2 = 1 - alpha,
            };
        }

        public static Biquad BandPass(double freq, double sampleRate, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = alpha,
                B1 = 0,
                B2 = -alpha,
                A0 = 1 + alpha,
                A1 = -2 * cos,
                A2 = 1 - alpha,
            };
        }

        public static Biquad AllPass(double freq, double sampleRate, double q)
        {
            double w0 = Omega(freq, sampleRate);
            double alpha = Math.Sin(w0) / (2.0 * q);
            double cos = Math.Cos(w0);

            return new Biquad
            {
                B0 = 1 - alpha,
                B1 = -2 * cos,
                B2 = 1 + alpha,
                A0 = 1 + alpha,
                A1 = -2 * cos,
                A2 = 1 - alpha,
            };
        }

        private static double Omega(double freq, double sampleRate)
        {
            double f = Math.Max(1.0, Math.Min(freq, sampleRate * 0.4999));
            return 2.0 * Math.PI * f / sampleRate;
        }

        public static double[] ButterworthQs(int order)
        {
            if (order < 2) order = 2;
            if (order % 2 != 0) order++;
            int sections = order / 2;
            var qs = new double[sections];
            for (int k = 0; k < sections; k++)
                qs[k] = 1.0 / (2.0 * Math.Cos((2.0 * k + 1.0) * Math.PI / (2.0 * order)));
            return qs;
        }
    }
}
